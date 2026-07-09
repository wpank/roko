# PRD-10: Dashboard and TUI -- unified surfaces for the Nunchi network

*Two mediums. One data contract. Near-zero latency between what the network knows and what the operator sees.*

**Status:** Draft
**Author:** Will
**Date:** 2026-04-21
**Surfaces:** `nunchi-dashboard` (React 19 SPA), `crates/roko-cli/src/tui/` (ratatui)
**Backends:** `roko-serve` (~85 routes, :6677), Nexus WebSocket relay, mirage-rs chain API
**Cross-references:** PRD-02 (agent runtime, heartbeats), PRD-05 (InsightStore, stigmergy), PRD-07 (ISFR instruments), PRD-08 (CLI DX, deployment), PRD-06 (domains, arenas)

---

## Table of contents

1. [Executive summary](#1-executive-summary)
2. [Design principles](#2-design-principles)
3. [Terminology](#3-terminology)
4. [Architecture overview](#4-architecture-overview)
5. [Nexus -- agent relay redesign](#5-nexus----agent-relay-redesign)
6. [Auth and identity](#6-auth-and-identity)
7. [Page catalog](#7-page-catalog)
   - 7.1 [Landing and onboarding](#71-landing-and-onboarding)
   - 7.2 [Command](#72-command-chat-and-research)
   - 7.3 [Observatory](#73-observatory)
   - 7.4 [Network](#74-network)
   - 7.5 [Marketplace](#75-marketplace-and-jobs)
   - 7.6 [Agent Studio](#76-agent-studio)
   - 7.7 [Atelier](#77-atelier-workspace)
   - 7.8 [Settings](#78-settings)
8. [Widget and component catalog](#8-widget-and-component-catalog)
9. [Data contracts](#9-data-contracts)
10. [Network intelligence display](#10-network-intelligence-display)
11. [Jobs system integration](#11-jobs-system-integration)
12. [TUI-specific enhancements](#12-tui-specific-enhancements)
13. [Dashboard-specific enhancements](#13-dashboard-specific-enhancements)
14. [Roko stabilization requirements](#14-roko-stabilization-requirements)
15. [Demo requirements (Thursday)](#15-demo-requirements-thursday)
16. [Cross-references and open questions](#16-cross-references-and-open-questions)

---

## 1. Executive summary

Nunchi has two operator surfaces: a web dashboard and a terminal UI. Both must show the same data, support the same interactions, and stay within one refresh cycle of the truth. Today, neither does this well.

The dashboard (`nunchi-dashboard`) is a React 19 SPA contaminated with mock data. Mock agents, mock insights, mock stats -- all hardcoded and mixed into live responses. The layout is a fixed three-column grid that breaks below 1100px. The routing is a useState switch statement. Auth is a plaintext password. Fourteen polling timers fire concurrently without deduplication. Half the buttons have no click handlers. The ResearchPanel runs a simulated lifecycle using setTimeout chains that never touch the API result.

The TUI (ratatui, `crates/roko-cli/src/tui/`) is in better shape. Seven tabs (F1-F7) render real data from disk files and the roko-serve API. The ROSEDUST theme, PostFX pipeline, and atmosphere system all work. But F6 and F7 have declared sub-views (ProviderHealth, ModelComparison, EngramDag, EpisodeReplay, KnowledgeBrowse) that are never rendered. The plan tree ignores two of its parameters. The log view rebuilds its unified log O(N) per frame. There is no streaming -- the TUI polls 7 disk files and re-parses them each tick.

This PRD defines the target state for both surfaces. It introduces three concepts:

- **Atelier** -- a persistent, named orchestration workspace that replaces the ad-hoc "plan run" framing. `roko atelier start --name "feature-x"` creates a resumable context with its own PRD backlog, plan registry, episode log, and learning state.

- **Nexus** -- a WebSocket relay hub that replaces the current polling model. Agents connect outbound to Nexus. Dashboards and CLI tools connect to Nexus to reach any agent. JSON-RPC 2.0 over WebSocket with pub/sub rooms.

- **Heartbeats** -- periodic telemetry packets from persistent agents carrying cognitive metrics, cost data, learning rates, and capability declarations. Heartbeats are the primary data source for every real-time display.

Every page defined in section 7 maps to both a web route and a TUI tab. Every data source is mapped to a specific roko-serve route (existing or new). Every wireframe is drawn twice -- once for the browser, once for the terminal. An implementation agent should be able to build any page from this document without asking questions.

---

## 2. Design principles

### 2.1 One-to-one parity

The dashboard and TUI expose the same pages, the same data, and the same interactions wherever the medium allows. Switching between them should not break the operator's mental model. Parity is defined at the data layer. If the dashboard shows "agent count by domain, updated every 5 seconds," the TUI shows the same count, same staleness, same domain breakdown -- rendered as a table instead of a pie chart.

Where a medium makes exact parity impossible (force-directed graph in the terminal), the TUI presents the same semantic information in a different form (adjacency table with degree counts).

### 2.2 Adaptive information density

Human attention is the scarcest resource. Nunchi's agents operate in three regimes, and the display density adapts to match:

| Regime | Trigger | Information strategy |
|--------|---------|---------------------|
| **Cruise** | All agents healthy, no gate failures in past 10 min, costs within budget | Minimal. Aggregate metrics only. Status dots green. No per-agent detail. |
| **Volatile** | One or more agents failing gates, token burn rate spike >2x baseline, cost spike | Moderate. Affected agents expanded, healthy agents collapsed. Highlighted anomaly rows. |
| **Crisis** | Agent stuck >5 min, gate pipeline fully failing, cost runaway, orchestrator crash | Maximal. Full error traces, remediation suggestions, per-tick timeline, cost breakdown. Alert banners. |

The current regime comes from `CorticalState.regime` (PRD-02 section 8), exposed on `GET /api/status`. Both surfaces subscribe via the WS `regime_changed` event.

When regime escalates from cruise to volatile, the layout shifts automatically: stat cards compress, detail panels expand for affected agents, secondary navigation collapses. When it returns to cruise, the layout breathes back out. Transition duration: 300ms on dashboard (CSS transition), 2 ticks on TUI (border color interpolation).

### 2.3 Progressive disclosure

Every page follows a three-layer disclosure model:

- **Layer 0 -- Summary.** Visible without interaction. One number or status per concept. "12 agents online." "3 gate failures today." "ISFR: 1.032."
- **Layer 1 -- Detail.** One click or keypress reveals the breakdown: which 3 gates failed, on which tasks, at what time.
- **Layer 2 -- Trace.** One more interaction reveals the raw signal: full log output, the diff, the token budget per section, the HDC fingerprint.

Agents with no anomalies are collapsed to a single status dot. Agents with issues expand automatically. The operator can always force-expand or force-collapse any entity. Default density matches the current regime.

### 2.4 Perpetual motion (TUI)

The TUI is always alive. Even when no agents are running, the interface breathes: border colors pulse along the ROSEDUST palette, background particles drift, the master clock ticks visibly. Static terminal UIs feel dead. Perpetual motion communicates that the system is present and watching even when idle.

The practical implementation is a 60fps render loop with the PostFX pipeline (nerv_viz, particles, bloom, vignette, ambient_orbs, dream_atmosphere). Every metric carries a sparkline showing its history even in cruise mode.

### 2.5 Math-to-metaphor translation

Nunchi's internals involve precise mathematical constructs: hyperdimensional computing, VCG auctions, Gittins index foraging, ISFR dual-median aggregation, Ebbinghaus curves. These are load-bearing concepts. They are also opaque to most users.

The interfaces translate them at layer 0, preserving the precise definition at layer 2:

| Technical concept | Layer 0 metaphor | Layer 2 truth |
|-------------------|-----------------|----------------|
| T0/T1/T2 cognitive gating | "Brain frequency" (gamma/theta/delta) | Tier routing decision tree with prediction error threshold |
| HDC fingerprint similarity | "Knowledge match %" | Cosine similarity of 10,240-bit hyperdimensional vectors |
| ISFR | "Network intelligence score" | Dual-median aggregation across validators with Byzantine fault tolerance |
| Ebbinghaus decay | "Memory freshness" | Exponential decay with kind-specific half-life |
| VCG auction context allocation | "Attention budget" | Vickrey-Clarke-Groves mechanism over 9 context section bidders |

### 2.6 One source of truth

Neither surface owns data or business logic. Both are views over the same API. No client-side state diverges from server state. No optimistic UI updates that are not immediately confirmed. No duplicate computation on the client.

Real-time data flows through WebSocket events. Polling is used only where WS is unavailable or staleness tolerance is high. The roko-serve API is the single source of truth. Nexus is the relay layer, not a cache.

---

## 3. Terminology

### Atelier

An Atelier is a persistent, named orchestration workspace. The word means "a workshop where craftspeople create work." Agents in an Atelier share a PRD backlog, a plan registry, an episode log, and a learning context. State persists across restarts via `.roko/state/executor.json`.

```bash
roko atelier start --name "feature-x"   # Create and start
roko atelier status                      # Show workspace status
roko atelier stop                        # Gracefully shut down
roko atelier list                        # List all ateliers
```

Contrast with a **harness** (`roko run "<prompt>"`), which is ephemeral: one prompt, one agent, one gate check, no persistence.

### Harness

A harness is an ephemeral execution context for a single-prompt agent run. No persistent state, no plan registry, no PRD backlog. The correct primitive for scripted invocations, CI integration, and quick one-off tasks.

### Nexus

The WebSocket relay hub that connects agents, dashboards, and CLI tools. Agents connect outbound to Nexus using their credentials; dashboards and CLI tools connect to Nexus to subscribe to agent streams and send messages. Nexus aggregates heartbeats, maintains a presence directory, and routes messages.

Nunchi operates a public Nexus instance at `nexus.nunchi.trade`. Self-hostable for private deployments. Protocol: JSON-RPC 2.0 over WebSocket with pub/sub rooms.

### Agent Passport

On-chain identity for a persistent agent, conforming to ERC-8004:
- Wallet address (EOA or smart contract account)
- Capability manifest (skills, domains, extensions)
- Stake balance (determines tier: Probation / Standard / Trusted / Elite)
- Reputation score (EMA over verified task outcomes)
- Public key for message verification

Agents without wallets can participate using API keys through Nexus. Their identity is ephemeral but their heartbeats and task outcomes still flow through the system.

### Heartbeat

Periodic telemetry from every persistent agent, on a configurable interval (default 10s):

```typescript
interface Heartbeat {
  agent_id: string;
  timestamp: number;          // Unix ms
  status: "idle" | "working" | "sleeping" | "error";
  current_task?: string;

  // Cognitive metrics
  cognitive_tier: "T0" | "T1" | "T2";
  context_utilization: number;   // 0.0-1.0
  token_burn_rate: number;       // tokens/minute

  // Learning metrics
  episode_count: number;
  gate_pass_rate: number;        // rolling 24h
  playbook_size: number;
  insight_count: number;

  // Cost metrics
  cumulative_cost_usd: number;
  burn_rate_usd_per_hour: number;

  // Capabilities
  skills: string[];
  extensions: string[];
  domain: string;
  model: string;
}
```

Heartbeats flow from agents to Nexus, which aggregates them and forwards to subscribed dashboards and CLI tools.

### Cognitive frequencies

The three processing tiers used in roko's cognitive architecture, mapped to human-readable metaphors:

| Tier | Metaphor | Brain wave | When used |
|------|----------|------------|-----------|
| T0 | Fast path | Gamma | Pattern-matched from existing knowledge. No LLM call needed. |
| T1 | Slow path | Theta | Requires reasoning but stays within familiar territory. Standard LLM call. |
| T2 | Deep path | Delta | Novel problem requiring extended reasoning, multi-step planning, or research. Expensive LLM call with expanded context. |

---

## 4. Architecture overview

### 4.1 System topology

```
+-------------------+     +-------------------+     +-------------------+
|   Dashboard       |     |   TUI (ratatui)   |     |   External        |
|   React 19 SPA    |     |   roko dashboard  |     |   Consumers       |
+--------+----------+     +--------+----------+     +--------+----------+
         |                          |                          |
         |   HTTPS / WSS           |   HTTP / WS              |   HTTP
         v                          v                          v
+--------+----------+     +--------+----------+     +--------+----------+
|                   |     |                   |     |                   |
|      Nexus        +-----+   roko-serve      +-----+   mirage-rs      |
|   (WS relay)      |     |   (:6677)         |     |   (chain API)    |
|                   |     |                   |     |                   |
+--------+----------+     +--------+----------+     +-------------------+
         |                          |
         |   WS outbound            |   Process management
         v                          v
+--------+----------+     +--------+----------+
|   Agent Sidecar   |     |   Orchestrator    |
|   (per-agent HTTP |     |   (orchestrate.rs)|
|    :random port)  |     |   PlanRunner      |
+-------------------+     +-------------------+
         |                          |
         v                          v
+-------------------+     +-------------------+
|   .roko/ data     |     |   LLM backends    |
|   JSONL files     |     |   Claude, Codex,  |
|   State, Learn    |     |   Gemini, Ollama  |
+-------------------+     +-------------------+
```

### 4.2 Data flow

All operator-facing data originates in one of four places:

1. **`.roko/` directory** -- JSONL files (episodes, signals, efficiency events), JSON files (cascade-router, gate-thresholds, experiments), plan files (TOML/JSON). roko-serve reads these on every request. No database.

2. **In-memory state in roko-serve** -- active plans, discovered agents, process supervisor entries, event bus ring buffer. Lost on restart. The AppState struct holds all of it behind Arc and RwLock.

3. **Agent sidecars** -- each running agent has an HTTP server (roko-agent-server) on a random port. roko-serve's aggregator routes fan out to these sidecars and cache responses with TTLs (5-30s depending on endpoint).

4. **On-chain state** -- BountyMarket, WorkerRegistry, ConsortiumValidator, JobTypeRegistry, DAEJI token. Read via mirage-rs chain API. Write via wallet transactions.

Both surfaces consume roko-serve. The dashboard does so directly over HTTP/WS. The TUI currently reads some data from disk files and some from roko-serve; the target state is full migration to roko-serve with WS streaming.

### 4.3 Auth flow

```
Dashboard:                    CLI:
  Privy SDK                     roko login
    |                             |
    v                             v
  Wallet signature              API key from roko-serve
    |                             |
    v                             v
  Session token                 Bearer token
    |                             |
    v                             v
  roko-serve auth middleware    roko-serve auth middleware
    |                             |
    v                             v
  Same API, same data           Same API, same data
```

---

## 5. Nexus -- agent relay redesign

### 5.1 Protocol

JSON-RPC 2.0 over WebSocket. All messages are JSON text frames. Binary frames are rejected.

**Request format:**

```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "method": "nexus.subscribe",
  "params": {
    "rooms": ["atelier:feature-x", "domain:trading"],
    "cursor": 0
  }
}
```

**Response format:**

```json
{
  "jsonrpc": "2.0",
  "id": "req-001",
  "result": {
    "subscribed": ["atelier:feature-x", "domain:trading"],
    "cursor": 42
  }
}
```

**Notification format (server-push):**

```json
{
  "jsonrpc": "2.0",
  "method": "nexus.event",
  "params": {
    "room": "atelier:feature-x",
    "type": "heartbeat",
    "agent_id": "agent-7",
    "data": { ... },
    "seq": 43
  }
}
```

### 5.2 Methods

| Method | Direction | Description |
|--------|-----------|-------------|
| `nexus.auth` | client -> server | Authenticate with wallet signature or API key |
| `nexus.subscribe` | client -> server | Subscribe to rooms with optional cursor for replay |
| `nexus.unsubscribe` | client -> server | Unsubscribe from rooms |
| `nexus.publish` | client -> server | Publish an event to a room |
| `nexus.presence` | client -> server | Query who is online in a room |
| `nexus.heartbeat` | agent -> server | Submit a heartbeat (agents only) |
| `nexus.message` | client -> server | Send a direct message to an agent |
| `nexus.event` | server -> client | Push event to subscribers |
| `nexus.presence_update` | server -> client | Agent joined/left room |
| `nexus.error` | server -> client | Error notification |

### 5.3 Rooms

Rooms scope message delivery. A client subscribes to one or more rooms and receives events published to those rooms.

| Room pattern | Example | Who publishes | Who subscribes |
|-------------|---------|---------------|----------------|
| `atelier:{name}` | `atelier:feature-x` | Agents in that workspace | Dashboard, TUI, other agents |
| `domain:{name}` | `domain:trading` | Agents with that domain tag | Network views, domain dashboards |
| `global` | `global` | System events, announcements | Everyone |
| `agent:{id}` | `agent:agent-7` | That specific agent | Agent detail views |
| `jobs` | `jobs` | BountyMarket contract events | Marketplace views |

### 5.4 Heartbeat aggregation

Nexus aggregates heartbeats from all connected agents. Subscribers to a room receive individual heartbeats from agents in that room. Subscribers to `global` receive a periodic aggregate:

```json
{
  "method": "nexus.event",
  "params": {
    "room": "global",
    "type": "heartbeat_aggregate",
    "data": {
      "total_agents": 47,
      "by_status": { "working": 31, "idle": 12, "sleeping": 3, "error": 1 },
      "by_domain": { "trading": 18, "research": 12, "infra": 9, "general": 8 },
      "total_burn_rate_usd_per_hour": 14.32,
      "average_gate_pass_rate": 0.847,
      "average_context_utilization": 0.62
    },
    "seq": 100
  }
}
```

Aggregate interval: 10 seconds (configurable).

### 5.5 Auth

Two auth methods, both prove identity before any other method is accepted:

**Wallet signature (agents with passports):**

```json
{
  "method": "nexus.auth",
  "params": {
    "type": "wallet",
    "address": "0x...",
    "message": "nexus-auth:1714300000:random-nonce",
    "signature": "0x..."
  }
}
```

**API key (dashboard, CLI, agents without wallets):**

```json
{
  "method": "nexus.auth",
  "params": {
    "type": "api_key",
    "key": "roko_..."
  }
}
```

### 5.6 Self-hosting

Nexus is shipped as a standalone binary (`roko-nexus`) and as a Dockerfile. Private deployments set `nexus.url` in `roko.toml` to point to their own instance. No data leaves the private network.

### 5.7 Network stats computation

Nexus computes the following from connected agents and publishes them on the `global` room:

- **Total agent count** (by status, domain, tier, model)
- **Network burn rate** (aggregate USD/hour across all connected agents)
- **Average gate pass rate** (rolling 24h across all agents)
- **ISFR** (dual-median aggregation, forwarded from mirage-rs)
- **Knowledge graph density** (edges/node ratio from aggregated knowledge endpoints)

---

## 6. Auth and identity

### 6.1 Dashboard auth -- Privy

The dashboard uses Privy for wallet-based authentication. When `VITE_PRIVY_APP_ID` is set, the Privy SDK handles login, wallet connection, and session management. The session token is sent as a Bearer token on all roko-serve requests.

**Current problems to fix:**
- Remove the hardcoded password fallback ("daeji"). Privy-only auth.
- Move from sessionStorage to localStorage for session persistence across browser tabs.
- Add session expiry (24 hours, configurable).

### 6.2 CLI auth -- `roko login`

```bash
roko login                    # Opens browser, Privy login, callback to localhost
roko login --api-key          # Prompt for API key directly
roko login --status           # Show current auth state
roko logout                   # Clear credentials
```

The CLI stores credentials in `~/.roko/credentials.json` (file permissions 0600). The API key is sent as a Bearer token on roko-serve requests. The TUI reads from the same credential store.

### 6.3 API key management

API keys are managed through roko-serve. They are scoped to a workspace or global.

| Route | Method | Description | Status |
|-------|--------|-------------|--------|
| `/api/keys` | GET | List API keys (redacted) | **[new]** |
| `/api/keys` | POST | Create API key | **[new]** |
| `/api/keys/{id}` | DELETE | Revoke API key | **[new]** |
| `/api/keys/{id}/rotate` | POST | Rotate API key | **[new]** |

### 6.4 Agent identity

Agents authenticate to roko-serve via `POST /api/agents/register` **[existing]** which can optionally issue a bearer token. For Nexus, agents use either wallet signatures (if they have a passport) or the API key issued at registration.

### 6.5 roko-serve auth middleware upgrade

Current state: a single API key checked in middleware when `serve.auth.enabled = true`. No per-key scoping, no rate limiting, no audit log.

Target state:

| Feature | Description |
|---------|-------------|
| Multi-key support | Multiple API keys, each with a label and optional scope |
| Key scoping | Read-only, read-write, admin. Per-route matching. |
| Rate limiting | Per-key rate limit (default: 100 req/s, configurable) |
| Audit log | Append to `.roko/audit.jsonl` on every authenticated request |
| Key rotation | Rotate without downtime (old key valid for 5 min overlap) |

---

## 7. Page catalog

Each page below includes both dashboard and TUI specifications. The format for each page:

- **Purpose:** one sentence
- **TUI mapping:** which tab/sub-view/keybinding
- **Dashboard route:** URL path
- **Wireframes:** ASCII art for both surfaces
- **Data sources:** exact roko-serve routes, poll intervals, WS events
- **Component hierarchy:** React components and their props/state
- **State management:** what lives where
- **Interactions:** user actions and their effects
- **Loading / empty / error states**
- **Mock data tags:** what must be removed from the current dashboard

---

### 7.1 Landing and onboarding

#### 7.1.1 Landing page

**Purpose:** First impression for new users -- explain what Nunchi is and how it works.
**TUI mapping:** Not applicable (TUI users already installed the CLI).
**Dashboard route:** `/`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| nunchi                                    [Connect Wallet]  [Docs]    |
+----------------------------------------------------------------------+
|                                                                      |
|                                                                      |
|        Agents that build themselves.                                 |
|                                                                      |
|        An orchestration toolkit for autonomous coding agents.        |
|        18 crates. ~177K lines of Rust. Fully self-hosting.           |
|                                                                      |
|        [Get Started]           [View Network]                        |
|                                                                      |
+----------------------------------------------------------------------+
|                                                                      |
|    +------------------+  +------------------+  +------------------+  |
|    | Plan             |  | Execute          |  | Learn            |  |
|    |                  |  |                  |  |                  |  |
|    | PRDs generate    |  | Agents run tasks |  | Gate results     |  |
|    | implementation   |  | with safety      |  | feed back into   |  |
|    | plans with DAG   |  | gates at every   |  | model routing    |  |
|    | task ordering    |  | step             |  | and context       |  |
|    +------------------+  +------------------+  +------------------+  |
|                                                                      |
+----------------------------------------------------------------------+
|    Live network: 47 agents | 12 domains | ISFR: 1.032               |
+----------------------------------------------------------------------+
```

##### Data sources

- Live network stats: `GET /api/status` **[existing]** (poll 30s) or Nexus `global` room `heartbeat_aggregate`
- ISFR: forwarded from mirage-rs chain API

##### Component hierarchy

```
- LandingPage
  - Hero (tagline, CTA buttons)
  - FeatureCards (3 cards: Plan, Execute, Learn)
  - NetworkTicker (agent count, domain count, ISFR)
```

##### State

- `isAuthenticated: boolean` -- from Privy
- `networkStats: { agents: number, domains: number, isfr: number }` -- from status endpoint

##### Interactions

- Click "Get Started" -> if not authenticated, Privy login flow. If authenticated, navigate to `/observatory`.
- Click "View Network" -> navigate to `/network`.
- Click "Connect Wallet" -> Privy wallet connection.

##### Loading / empty / error states

- Loading: network ticker shows skeleton pulse
- Error: network ticker shows "Network unavailable" with last-known values

##### Mock data to remove

- Remove hardcoded "47" agent count, "45,231" block, "6.40%" ISFR from current App.tsx top nav.

---

#### 7.1.2 Onboarding wizard

**Purpose:** Guide new users through initial setup -- connect wallet, install CLI, start first atelier.
**TUI mapping:** `roko init` (already exists).
**Dashboard route:** `/onboarding`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| Step 2 of 4: Install the CLI                                         |
+----------------------------------------------------------------------+
|                                                                      |
|   [1 Connect] -- [2 Install] -- [3 Configure] -- [4 First Run]      |
|                      ^                                               |
|                      |                                               |
|   +----------------------------------------------------------+      |
|   |                                                          |      |
|   |   curl -sSL https://get.nunchi.trade | sh                |      |
|   |                                                          |      |
|   |   # Or via cargo:                                        |      |
|   |   cargo install roko-cli                                 |      |
|   |                                                          |      |
|   |   # Verify:                                              |      |
|   |   roko --version                                         |      |
|   |                                                          |      |
|   +----------------------------------------------------------+      |
|   [Copy]                                                             |
|                                                                      |
|   Once installed, run: roko login                                    |
|                                                                      |
|              [Back]                      [Next: Configure]           |
+----------------------------------------------------------------------+
```

##### Data sources

- Step 1 (Connect): Privy SDK, no roko-serve calls
- Step 3 (Configure): `PUT /api/config` **[existing]** to write initial roko.toml
- Step 4 (First Run): `POST /api/plans/generate` **[existing]** to verify the pipeline

##### Component hierarchy

```
- OnboardingWizard
  - StepIndicator (current step, total steps)
  - StepContent
    - ConnectStep (Privy wallet)
    - InstallStep (code block with copy button)
    - ConfigureStep (model selector, provider API keys)
    - FirstRunStep (create demo atelier)
  - NavigationButtons (Back, Next)
```

##### State

- `currentStep: 1-4`
- `walletConnected: boolean`
- `cliInstalled: boolean` (verified via `GET /api/health` after CLI install)
- `configSaved: boolean`

##### Interactions

- Complete step 1 -> Privy returns wallet, auto-advance to step 2
- Step 3 submit -> `PUT /api/config` with model provider credentials
- Step 4 "Create demo" -> `POST /api/plans/generate` with slug `onboarding-demo`

##### Loading / empty / error states

- Loading: spinner on "Next" button during API calls
- Error: inline error below the step content with retry

---

### 7.2 Command (chat and research)

#### 7.2.1 Ask / Chat

**Purpose:** Send prompts to agents and view their responses in a conversational interface.
**TUI mapping:** `roko chat --agent <id>` (existing CLI command). Not yet a TUI tab -- add as F8 or integrate into F1 sub-tab.
**Dashboard route:** `/command/chat`
**Component:** `ChatPage`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Command]     Ask                          [Agent: implementer v] |
+----------------------------------------------------------------------+
|                                                                      |
|  +----------------------------------------------------------------+  |
|  |                                                                |  |
|  |  You: Explain the gate pipeline                                |  |
|  |                                                                |  |
|  |  Agent (implementer): The gate pipeline has 7 rungs (0-6),     |  |
|  |  each progressively stricter. Rung 0 checks basic syntax.      |  |
|  |  Rung 1 runs `cargo check`. Rung 2 runs `cargo clippy`.        |  |
|  |  ...                                                           |  |
|  |                                                                |  |
|  |  Tokens: 847 in / 1,203 out | Cost: $0.012 | Model: opus-4    |  |
|  |                                                                |  |
|  |  You: Now implement it in task T7                              |  |
|  |                                                                |  |
|  |  Agent (implementer): Working on T7...                         |  |
|  |  [streaming response...]                                      |  |
|  |                                                                |  |
|  +----------------------------------------------------------------+  |
|                                                                      |
|  +----------------------------------------------------------------+  |
|  | Type a message...                              [Send] [Ctrl+E] |  |
|  +----------------------------------------------------------------+  |
|                                                                      |
|  Context: atelier:feature-x | Conversation: conv-001               |
+----------------------------------------------------------------------+
```

##### Wireframe (TUI)

```
+----------------------------------------------------------------------+
| F1 Dash | F2 Plans | F3 Agents | F4 Git | F5 Logs | F6 Cfg | F7 Ins |
| F8 Chat                                                              |
+----------------------------------------------------------------------+
| Agent: implementer (opus-4)     | atelier:feature-x | conv-001      |
+----------------------------------+-----------------------------------+
|                                  |                                   |
| > Explain the gate pipeline      | Context                          |
|                                  | --------                         |
| [implementer] The gate pipeline  | Model: opus-4                    |
| has 7 rungs (0-6), each         | Tokens: 847/1203                 |
| progressively stricter...        | Cost: $0.012                     |
|                                  | Gate pass: 87%                   |
| > Now implement it in T7         | Episodes: 42                     |
|                                  |                                   |
| [implementer] Working on T7...   |                                   |
| [streaming...]                   |                                   |
|                                  |                                   |
+----------------------------------+-----------------------------------+
| > _                                                        [Enter]  |
+----------------------------------------------------------------------+
```

##### Data sources

- Agent list: `GET /api/agents` **[existing]** (poll 30s)
- Send message: `POST /api/agents/{id}/message` **[existing]**
- Streaming response: WS `/ws` **[existing]** -- subscribe to `AgentOutput` events filtered by agent_id
- Agent stats: `GET /api/agents/{id}/stats` **[existing]** via aggregator (poll 10s)
- Conversation history: `GET /api/agents/{id}/episodes` **[existing]**

##### Component hierarchy

```
- AskPage
  - AgentSelector (dropdown, current agent)
  - MessageList
    - UserMessage (text, timestamp)
    - AgentMessage (text, timestamp, metadata: tokens, cost, model)
    - StreamingMessage (partial text, cursor animation)
  - InputBar (textarea, send button, keyboard shortcut hint)
  - ContextSidebar (agent stats, conversation metadata)
```

##### State

- `selectedAgentId: string`
- `conversationId: string` -- generated on first message, reused for context
- `messages: Array<{ role: "user" | "agent", text: string, meta?: MessageMeta }>`
- `isStreaming: boolean`
- `inputText: string`

##### Interactions

- Select agent from dropdown -> updates selectedAgentId, loads that agent's recent episodes
- Type message, press Enter or click Send -> `POST /api/agents/{id}/message` with `{ message, conversation_id }`
- Response streams via WS AgentOutput events -> append to messages as StreamingMessage, convert to AgentMessage on completion
- Press Ctrl+E -> toggle expanded input (multi-line)
- Press Escape while streaming -> cancel (future: implement via agent stop endpoint)

##### Loading / empty / error states

- Loading: skeleton cards in message list while loading conversation history
- Empty: "Start a conversation with an agent. Select one above and type your message."
- Error: inline banner below message list: "Failed to send message. [Retry]"

##### Mock data to remove

- AskPanel `cacheAgentEndpoint` (empty function)
- `msg.type === "tx_lookup"` dead code branch
- OpenRouter inference -- replace with roko-serve agent message endpoint

---

#### 7.2.2 Research

**Purpose:** Launch deep research tasks and view their results.
**TUI mapping:** No dedicated TUI tab currently. Add as sub-tab under F8 Chat, or use `roko research topic` CLI command.
**Dashboard route:** `/command/research`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Command]     Research                                            |
+----------------------------------------------------------------------+
|                                                                      |
|  +-------------------------------+  +-----------------------------+  |
|  | New research                  |  | Recent research             |  |
|  |                               |  |                             |  |
|  | Topic: [________________]     |  | * model-routing.md   2h ago|  |
|  |                               |  |   Intent: position          |  |
|  | Intent: [position      v]     |  |   Status: complete          |  |
|  |                               |  |                             |  |
|  | [Start Research]              |  | * gate-pipeline.md   1d ago|  |
|  |                               |  |   Intent: evaluate          |  |
|  +-------------------------------+  |   Status: complete          |  |
|                                     |                             |  |
|  +-------------------------------+  | * erc-8183.md        3d ago|  |
|  | PRD enhancement              |  |   Intent: explore           |  |
|  |                               |  |   Status: complete          |  |
|  | PRD: [system-prompt v]        |  |                             |  |
|  | [Enhance PRD]                 |  +-----------------------------+  |
|  +-------------------------------+                                   |
|                                                                      |
+----------------------------------------------------------------------+
| Selected: model-routing.md                                           |
+----------------------------------------------------------------------+
| # Model Routing Research                                             |
| ## Findings                                                          |
| 1. LinUCB performs well for cold-start but...                        |
| ...                                                                  |
+----------------------------------------------------------------------+
```

##### Data sources

- Research artifacts: `GET /api/research` **[existing]** (poll 30s)
- Start topic research: `POST /api/research/topic` **[existing]**
- Enhance PRD: `POST /api/research/enhance-prd/{slug}` **[existing]**
- Enhance plan: `POST /api/research/enhance-plan/{plan}` **[existing]**
- PRD list (for dropdown): `GET /api/prds` **[existing]**
- Operation status: listen for `OperationCompleted` on WS `/ws` **[existing]**
- Read artifact content: `GET /api/research/{name}` **[new -- needs file-read endpoint]**

##### Component hierarchy

```
- ResearchPage
  - ResearchForm
    - TopicInput (text field)
    - IntentSelector (dropdown: position, evaluate, monitor, explore, audit)
    - SubmitButton
  - PrdEnhancementForm
    - PrdSelector (dropdown, populated from /api/prds)
    - EnhanceButton
  - RecentResearchList
    - ResearchItem (name, intent, status, age)
  - ResearchViewer (markdown renderer for selected artifact)
```

##### State

- `artifacts: Array<{ name, size, is_file }>` -- from `GET /api/research`
- `selectedArtifact: string | null`
- `artifactContent: string` -- loaded on selection
- `activeOperations: Map<string, { kind, status }>` -- tracked via WS events
- `topicInput: string`
- `selectedIntent: string`
- `selectedPrd: string`

##### Interactions

- Submit topic -> `POST /api/research/topic` -> operation ID returned -> listen on WS for completion -> refresh artifact list
- Select artifact -> fetch content, render in markdown viewer
- Enhance PRD -> `POST /api/research/enhance-prd/{slug}` -> same operation tracking
- Click artifact in recent list -> load and display

##### Loading / empty / error states

- Loading: spinner on submit buttons while operation runs. Research list shows "Researching..." with progress indicator.
- Empty: "No research artifacts yet. Start by entering a topic above."
- Error: inline error below the form that triggered it

##### Mock data to remove

- Entire ResearchPanel setTimeout lifecycle simulation
- `researchTopic()` result is currently ignored -- wire it properly

---

### 7.3 Observatory

The Observatory is the primary monitoring section. It shows what is happening right now across all agents, plans, and the learning system.

#### 7.3.1 Live agents

**Purpose:** Real-time view of all running agents with status, cognitive tier, token burn, and cost.
**TUI mapping:** F1 Dashboard sub-tab `a:Agents` + F3 Agents view
**Dashboard route:** `/observatory/agents`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Observatory]  Live Agents                    [Cruise | 12 agents]|
+----------------------------------------------------------------------+
|                                                                      |
| +-----+-----+-------+------+------+-------+--------+------+-------+ |
| | ID  |Role | Model | Tier | Task | Burn  | Cost   | Gate | Ctx   | |
| +-----+-----+-------+------+------+-------+--------+------+-------+ |
| |ag-1 |impl | opus  | T1   | T-04 | 2.1k  | $1.23  | 92%  | 67%  | |
| |     |     |       |theta |      | tok/m  | /hr    |      |      | |
| +-----+-----+-------+------+------+-------+--------+------+-------+ |
| |ag-2 |arch | sonnet| T0   | T-12 | 0.4k  | $0.18  | 100% | 34%  | |
| |     |     |       |gamma |      | tok/m  | /hr    |      |      | |
| +-----+-----+-------+------+------+-------+--------+------+-------+ |
| |ag-3 |res  | opus  | T2   | --   | 4.8k  | $3.42  | 78%  | 91%  | |
| |     |     |       |delta |      | tok/m  | /hr    |[!]   |      | |
| +-----+-----+-------+------+------+-------+--------+------+-------+ |
|                                                                      |
| Total burn: $4.83/hr | Avg gate: 90% | Tokens: 7.3k/min            |
+----------------------------------------------------------------------+
|                                                                      |
| ag-3 Detail (expanded -- volatile agent auto-expands)                |
| +----------------------------------------------------------------+  |
| | Current task: (none -- idle)                                   |  |
| | Last gate failure: rung 4 (clippy) on task T-09, 8 min ago    |  |
| | Episode count: 142 | Playbook size: 23 | Insights: 7          |  |
| | Context utilization: [=================== ] 91%                |  |
| | Token sparkline (1h): __|__|'''|---|^^^|---|'''|                |  |
| +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Wireframe (TUI)

```
+----------------------------------------------------------------------+
| F1 Dash | F2 Plans | F3 Agents | F4 Git | F5 Logs | F6 Cfg | F7 Ins |
+-------- a:Agents  o:Out  d:Diff  e:Gate  g:Git  m:MCP  L:Learn  P:--+
+-------------------------------+--------------------------------------+
| PLAN TREE                     | AGENTS          [cruise]  12 agents  |
|                               |                                      |
| v feature-x [=====>   ] 67%  | ag-1 impl opus  T1 T-04 2.1k $1.23  |
|   T-01 draft-prd      [done] | ag-2 arch snnt  T0 T-12 0.4k $0.18  |
|   T-02 research        [done]| ag-3 res  opus  T2 idle 4.8k $3.42 !|
|   T-03 plan-gen        [done]|                                      |
|   T-04 implement     [active]| --- ag-3 expanded (gate failure) --- |
|   T-05 gate           [pend] | Last fail: rung4/clippy T-09 8m ago  |
|   T-06 review          [pend]| Episodes:142 Playbooks:23 Insights:7 |
|                               | Ctx: [=================== ] 91%    |
+-------------------------------+--------------------------------------+
| WAVE [==============>         ] 67%  |  tok/m: 7.3k  | $4.83/hr    |
+----------------------------------------------------------------------+
```

##### Data sources

- Agent roster: `GET /api/agents` **[existing]** (poll 10s) -- aggregator route, fans out to sidecars
- Agent topology: `GET /api/agents/topology` **[existing]** (poll 30s)
- Agent stats per agent: `GET /api/agents/{id}/stats` **[existing]** via aggregator (poll 10s)
- Agent heartbeat per agent: `GET /api/agents/{id}/heartbeat` **[existing]** via aggregator
- Real-time updates: WS `/ws` **[existing]** -- `AgentOutput`, `RunStarted`, `RunCompleted` events
- Regime: `GET /api/status` **[existing]** -- `regime` field
- Managed processes: `GET /api/managed-agents` **[existing]**

##### Component hierarchy

```
- LiveAgentsPage
  - RegimeBadge (cruise/volatile/crisis)
  - AgentTable
    - AgentRow (id, role, model, tier, task, burn_rate, cost, gate_rate, ctx_util)
    - ExpandedAgentDetail (detail panel for volatile/selected agents)
      - TaskInfo
      - GateFailureInfo
      - LearningMetrics (episode_count, playbook_size, insight_count)
      - ContextGauge (horizontal bar, percentage)
      - TokenSparkline (1h history)
  - AggregateBar (total burn, avg gate, total tokens/min)
```

##### State

- `agents: Map<string, AgentData>` -- keyed by agent_id
- `selectedAgentId: string | null` -- for forced expansion
- `regime: "cruise" | "volatile" | "crisis"` -- from status
- `expandedAgents: Set<string>` -- auto-populated by regime, manually toggleable
- `sortColumn: string` -- default "id"
- `sortDirection: "asc" | "desc"`

##### Interactions

- Click agent row -> toggle expanded detail
- Click column header -> sort by that column
- Regime change -> auto-expand agents with issues, collapse healthy ones (unless manually pinned)
- Click gate failure -> navigate to Observatory/Learning with that gate result selected
- Click task ID -> navigate to Observatory/Plans with that task selected

##### Loading / empty / error states

- Loading: table skeleton with 3 placeholder rows
- Empty: "No agents running. Start an atelier to see agents here."
- Error: banner above table: "Failed to fetch agent data. Showing last known state."

##### Mock data to remove

- Remove mock `agents` array from AgentOverview
- Remove "47" hardcoded agent count from top nav
- Remove all mock agent IDs ("agent-alpha", "agent-bravo", etc.)

---

#### 7.3.2 Plans

**Purpose:** View all plans, their task DAGs, execution state, wave progress, and gate results.
**TUI mapping:** F2 Plans
**Dashboard route:** `/observatory/plans`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Observatory]  Plans                                              |
+----------------------------------------------------------------------+
| +---------------------------+  +----------------------------------+  |
| | Plans                     |  | feature-x                        |  |
| |                           |  |                                  |  |
| | > feature-x    [67%] act |  | Title: Implement feature X       |  |
| |   infra-update [100%] don|  | Tasks: 6 total, 4 done, 1 active |  |
| |   gate-fixes   [  0%] pen|  | Duration: 2h 14m                 |  |
| |                           |  | Cost: $12.47                     |  |
| |                           |  |                                  |  |
| | +---------+               |  | +------------------------------+ |  |
| | |New Plan |               |  | | DAG                          | |  |
| | +---------+               |  | |                              | |  |
| +---------------------------+  | |  [T-01]-->[T-02]-->[T-03]   | |  |
|                                | |             |         |      | |  |
|                                | |             v         v      | |  |
|                                | |          [T-04]--->[T-05]    | |  |
|                                | |                       |      | |  |
|                                | |                       v      | |  |
|                                | |                    [T-06]    | |  |
|                                | +------------------------------+ |  |
|                                |                                  |  |
|                                | +------------------------------+ |  |
|                                | | Tasks                        | |  |
|                                | | T-01 draft-prd        [done]| |  |
|                                | | T-02 research          [done]| |  |
|                                | | T-03 plan-gen          [done]| |  |
|                                | | T-04 implement       [actv] | |  |
|                                | | T-05 gate            [pend] | |  |
|                                | | T-06 review           [pend]| |  |
|                                | +------------------------------+ |  |
|                                +----------------------------------+  |
+----------------------------------------------------------------------+
```

##### Wireframe (TUI)

```
+----------------------------------------------------------------------+
| F1 Dash | F2 Plans | F3 Agents | F4 Git | F5 Logs | F6 Cfg | F7 Ins |
+----------------------------------------------------------------------+
| PLANS                          | feature-x                          |
|                                |                                    |
| > feature-x    [==>    ] 67%  | Title: Implement feature X         |
|   infra-update [======] 100%  | Tasks: 6 (4 done, 1 active)        |
|   gate-fixes   [       ]  0%  | Duration: 2h 14m | Cost: $12.47    |
|                                |                                    |
|                                | DAG: T01->T02->T03                 |
|                                |       |    '->T04->T05             |
|                                |       '---------'->T06             |
|                                |                                    |
|                                | T-01 draft-prd        [done]      |
|                                | T-02 research          [done]     |
|                                | T-03 plan-gen          [done]     |
|                                | T-04 implement       > [active]   |
|                                | T-05 gate              [pending]  |
|                                | T-06 review            [pending]  |
+----------------------------------------------------------------------+
| WAVE [==============>         ] 67%  |  4/6 tasks                   |
+----------------------------------------------------------------------+
```

##### Data sources

- Plan list: `GET /api/plans` **[existing]** (poll 15s)
- Plan detail: `GET /api/plans/{id}` **[existing]** (on selection)
- Plan execution status: `GET /api/plans/{id}/status` **[existing]** (poll 5s while active)
- Plan events: WS `/ws` -- `PlanStarted`, `PlanCompleted` events **[existing]**
- Task-level gate results: `GET /api/metrics/gate_rate` **[existing]**
- Wave progress: computed client-side from plan tasks (done/total)

##### Component hierarchy

```
- PlansPage
  - PlanList
    - PlanItem (id, title, progress_bar, status_badge)
  - PlanDetail
    - PlanHeader (title, task_count, duration, cost)
    - DagView (SVG/canvas for dashboard, ASCII for TUI)
    - TaskList
      - TaskRow (id, description, status_badge, agent_id, gate_result)
  - WaveProgressBar (aggregate progress)
```

##### State

- `plans: Array<PlanSummary>` -- from plan list endpoint
- `selectedPlanId: string | null`
- `selectedPlan: PlanDetail | null` -- loaded on selection
- `activePlans: Set<string>` -- plans currently executing

##### Interactions

- Click plan in list -> load detail, show DAG and task list
- Click task in task list -> show task detail modal (description, files, gate results, agent output)
- Click "New Plan" -> navigate to plan creation form
- Click "Execute" on a plan -> `POST /api/plans/{id}/execute` -> track via WS events
- In TUI: `Enter` on plan to select, `j/k` to navigate tasks, `d` for task detail modal

##### Loading / empty / error states

- Loading: plan list skeleton, detail area empty
- Empty: "No plans found. Create one with `roko plan create` or from a PRD."
- Error: inline in plan detail area: "Failed to load plan."

---

#### 7.3.3 Learning

**Purpose:** Show how the system learns from experience -- cascade router decisions, gate adaptive thresholds, prompt experiments, efficiency metrics.
**TUI mapping:** F1 Dashboard sub-tab `L:Learning` + F7 Inspect
**Dashboard route:** `/observatory/learning`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Observatory]  Learning                                           |
+----------------------------------------------------------------------+
| +--------------------------------+  +-----------------------------+  |
| | Cascade router                 |  | Prompt experiments          |  |
| |                                |  |                             |  |
| | Model selection over time:     |  | Active: 2 | Completed: 5   |  |
| |                                |  |                             |  |
| | opus   [==========] 42%       |  | exp-01: system-prompt-v2    |  |
| | sonnet [=======   ] 35%       |  |   variant A: 87% pass       |  |
| | haiku  [====      ] 18%       |  |   variant B: 91% pass  [W] |  |
| | codex  [=         ]  5%       |  |   p-value: 0.032            |  |
| |                                |  |                             |  |
| | Cold-start threshold: 10      |  | exp-02: context-bidding     |  |
| | Total decisions: 847          |  |   running... 34/100 trials  |  |
| +--------------------------------+  +-----------------------------+  |
|                                                                      |
| +--------------------------------+  +-----------------------------+  |
| | Gate adaptive thresholds       |  | Efficiency events           |  |
| |                                |  |                             |  |
| | Rung | Current | EMA  | Trend |  | Last 24h:                   |  |
| | 0    | 0.95    | 0.94 | =     |  |  Total tasks: 47            |  |
| | 1    | 0.88    | 0.86 | ^     |  |  First-pass rate: 72%       |  |
| | 2    | 0.82    | 0.79 | ^     |  |  Avg tokens/task: 12,400    |  |
| | 3    | 0.71    | 0.74 | v     |  |  Cost/task: $0.87            |  |
| | 4    | 0.65    | 0.68 | v     |  |  Retries: 1.4 avg           |  |
| | 5    | 0.53    | 0.55 | =     |  |                             |  |
| | 6    | 0.41    | 0.43 | =     |  | C-Factor: 0.82             |  |
| +--------------------------------+  | Fleet C-Factor: 0.79       |  |
|                                     +-----------------------------+  |
+----------------------------------------------------------------------+
```

##### Wireframe (TUI -- F1 L:Learning sub-tab)

```
+----------------------------------------------------------------------+
| F1 Dash |- a  o  d  e  g  m  [L]  P -|                              |
+-------------------------------+--------------------------------------+
| PLAN TREE                     | LEARNING                             |
|                               |                                      |
| v feature-x [=====>   ] 67%  | CASCADE ROUTER                       |
|   T-01 draft-prd      [done] | opus  [==========] 42% (356 trials)  |
|   T-02 research        [done]| sonnet[=======   ] 35% (297 trials)  |
|   T-03 plan-gen        [done]| haiku [====      ] 18% (153 trials)  |
|   T-04 implement     [active]| codex [=         ]  5% ( 41 trials)  |
|   T-05 gate           [pend] |                                      |
|   T-06 review          [pend]| GATE THRESHOLDS                      |
|                               | R0:0.95 R1:0.88 R2:0.82 R3:0.71    |
|                               | R4:0.65 R5:0.53 R6:0.41            |
|                               |                                      |
|                               | EFFICIENCY (24h)                     |
|                               | Tasks:47 Pass:72% Tok:12.4k $/t:0.87|
|                               | C-Factor: 0.82 | Fleet: 0.79        |
+-------------------------------+--------------------------------------+
| WAVE [==============>         ] 67%  |  tok/m: 7.3k  | $4.83/hr    |
+----------------------------------------------------------------------+
```

##### Data sources

- Cascade router: `GET /api/learning/cascade-router` **[existing]** (poll 30s)
- Cascade decisions: `GET /api/learning/cascade` **[existing]**
- Cost tiers: `GET /api/learning/cost-tiers` **[existing]**
- Experiments: `GET /api/learning/experiments` **[existing]** (poll 30s)
- Efficiency events: `GET /api/learning/efficiency` **[existing]** (poll 15s)
- Adaptive thresholds: `GET /api/learning/adaptive-thresholds` **[existing]**
- C-Factor trend: `GET /api/c-factor/trend` **[existing]**
- C-Factor metrics: `GET /api/metrics/c_factor` **[existing]**
- Model efficiency: `GET /api/metrics/model_efficiency` **[existing]**

##### Component hierarchy

```
- LearningPage
  - CascadeRouterCard
    - ModelBar (model name, usage percentage, trial count)
    - ColdStartIndicator
  - ExperimentsCard
    - ExperimentRow (name, variant A/B results, p-value, winner badge)
    - RunningExperimentProgress
  - GateThresholdsCard
    - RungRow (rung number, current value, EMA, trend arrow)
  - EfficiencyCard
    - MetricRow (label, value)
    - CFactorGauge
    - FleetCFactorGauge
```

##### State

- `cascadeRouter: CascadeRouterState` -- model weights, trial counts
- `experiments: Array<Experiment>` -- from experiments endpoint
- `thresholds: Map<rung, { current, ema, trend }>` -- from adaptive thresholds
- `efficiency: EfficiencyMetrics` -- aggregated from efficiency events
- `cfactorTrend: Array<CFactorBucket>` -- time series

##### Interactions

- Click cascade model -> expand to show per-category/complexity breakdown
- Click experiment -> expand to show trial history
- Click C-Factor -> expand to show component breakdown (competence, context, coherence, etc.)

##### Loading / empty / error states

- Loading: card skeletons
- Empty: "No learning data yet. Run some tasks to start collecting metrics."
- Error: per-card inline error with retry

---

#### 7.3.4 Conductor

**Purpose:** Show the conductor subsystem's watchers, circuit breaker state, and diagnostics.
**TUI mapping:** F7 Inspect sub-view 5 (Conductor). See Task 3.3 in IMPL-10.
**Dashboard route:** `/observatory/conductor`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Observatory]  Conductor                                          |
+----------------------------------------------------------------------+
| +--------------------------------+  +-----------------------------+  |
| | Circuit breaker                |  | Watchers (10)               |  |
| |                                |  |                             |  |
| | State: CLOSED                  |  | FileSystem    [ok]  12ms   |  |
| | Failures: 0/5                  |  | Process       [ok]   3ms   |  |
| | Last trip: never               |  | Network       [ok]  89ms   |  |
| | Cooldown: 30s                  |  | Memory        [ok]   1ms   |  |
| |                                |  | Token         [ok]   0ms   |  |
| | [====================] healthy |  | Gate          [ok]   5ms   |  |
| +--------------------------------+  | Config        [ok]   2ms   |  |
|                                     | Episode       [ok]   1ms   |  |
| +--------------------------------+  | Schedule      [ok]   0ms   |  |
| | Diagnostics                    |  | Health        [ok]   4ms   |  |
| |                                |  +-----------------------------+  |
| | No active diagnoses.           |                                   |
| |                                |                                   |
| +--------------------------------+                                   |
+----------------------------------------------------------------------+
```

##### Data sources

- Diagnosis: `GET /api/diagnosis` **[existing]** (poll 15s)
- Health/metrics: `GET /api/health` **[existing]**, `GET /api/metrics` **[existing]**
- Status: `GET /api/status` **[existing]**

##### Component hierarchy

```
- ConductorPage
  - CircuitBreakerCard (state, failure count, last trip, cooldown)
  - WatcherList
    - WatcherRow (name, status badge, latency)
  - DiagnosticsPanel
    - DiagnosisCard (severity, message, timestamp, recommended action)
```

##### State

- `circuitBreaker: { state, failures, threshold, lastTrip, cooldown }`
- `watchers: Array<{ name, status, latency_ms }>`
- `diagnoses: Array<DiagnosisSummary>`

##### Interactions

- Click watcher -> expand to show recent check history
- Click diagnosis -> expand to show full diagnosis detail and recommended actions

##### Loading / empty / error states

- Loading: card skeletons
- Empty watchers: should not happen (watchers are hardcoded)
- Empty diagnoses: "System healthy. No active diagnoses."
- Error: banner: "Cannot reach conductor. Is roko-serve running?"

##### TUI

TUI: F7 Inspect sub-view 5. See Task 3.3 in IMPL-10.

---

#### 7.3.5 Costs

**Purpose:** Show cost breakdown by agent, model, task, and time period.
**TUI mapping:** F7 Inspect default sub-view (System Health -- token burn, cost by model sections). Already partially implemented.
**Dashboard route:** `/observatory/costs`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Observatory]  Costs                     [24h] [7d] [30d] [All]  |
+----------------------------------------------------------------------+
| +---------------------+  +---------------------+  +---------------+ |
| | Total spend         |  | Burn rate           |  | Budget        | |
| | $47.23              |  | $4.83/hr            |  | $100/day      | |
| | +12% vs yesterday   |  | trend: [sparkline]  |  | 47% used      | |
| +---------------------+  +---------------------+  +---------------+ |
|                                                                      |
| +----------------------------------------------------------------+  |
| | By model                                                       |  |
| |                                                                |  |
| | opus-4      $28.14 (60%)  [========================          ] |  |
| | sonnet-4    $12.37 (26%)  [==========                        ] |  |
| | haiku-3.5    $4.89 (10%)  [====                              ] |  |
| | codex        $1.83  (4%)  [==                                ] |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | By agent                                                       |  |
| |                                                                |  |
| | Agent   | Tasks | Tokens   | Cost    | $/task | Efficiency    |  |
| | ag-1    |    12 | 148,200  | $18.42  | $1.54  | 0.87          |  |
| | ag-2    |     8 |  42,100  |  $5.26  | $0.66  | 0.94          |  |
| | ag-3    |    27 | 189,400  | $23.55  | $0.87  | 0.82          |  |
| +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- Cost tiers: `GET /api/learning/cost-tiers` **[existing]**
- Efficiency events: `GET /api/learning/efficiency` **[existing]**
- Model efficiency: `GET /api/metrics/model_efficiency` **[existing]**
- C-Factor trend: `GET /api/c-factor/trend` **[existing]** (has cost buckets)
- Metrics summary: `GET /api/metrics/summary` **[existing]**
- Velocity: `GET /api/metrics/velocity` **[existing]**

##### Component hierarchy

```
- CostsPage
  - PeriodSelector (24h, 7d, 30d, all)
  - SummaryCards
    - TotalSpendCard
    - BurnRateCard (with sparkline)
    - BudgetCard (with progress bar)
  - ByModelBreakdown
    - ModelCostBar (model, cost, percentage, visual bar)
  - ByAgentTable
    - AgentCostRow (agent, tasks, tokens, cost, cost_per_task, efficiency)
```

##### State

- `period: "24h" | "7d" | "30d" | "all"`
- `costData: CostBreakdown` -- aggregated from multiple endpoints
- `burnRateHistory: Array<{ timestamp, rate }>` -- for sparkline

##### Interactions

- Click period selector -> re-fetch all data for that period
- Click agent row -> navigate to Observatory/Agents with that agent selected
- Click model name -> expand to show per-task breakdown for that model

##### Loading / empty / error states

- Loading: card skeletons, table skeleton
- Empty: "No cost data yet. Costs appear after agents complete tasks."
- Error: per-section inline error

##### TUI

TUI: F7 Inspect default sub-view (System Health section). Already partially implemented.

---

### 7.4 Network

#### 7.4.1 Agent network

**Purpose:** Visualize the topology of all agents -- who connects to whom, what domains they cover, their current status.
**TUI mapping:** F3 Agents (agent roster + topology data)
**Dashboard route:** `/network/topology`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Network]  Agent Network                  [Force] [Grid] [List]  |
+----------------------------------------------------------------------+
|                                                                      |
|           o-ag-1 (impl)                                              |
|          / \                                                         |
|    o-ag-2   o-ag-3 (res)                                             |
|    (arch)   / \                                                      |
|       \    /   \                                                     |
|        o-ag-4   o-ag-5                                               |
|        (audit)  (strat)                                              |
|                                                                      |
|  Legend: o = active  . = idle  x = error                             |
|  Color by: [Domain v]  Size by: [Token burn v]                       |
|                                                                      |
+----------------------------------------------------------------------+
| Selected: ag-3 | Role: researcher | Domain: trading | Tier: T2      |
| Skills: market-analysis, protocol-research | Gate: 78% | $3.42/hr   |
+----------------------------------------------------------------------+
```

##### Wireframe (TUI)

```
+----------------------------------------------------------------------+
| F3 Agents                                                            |
+----------------------------------------------------------------------+
| ROSTER                         | TOPOLOGY (adjacency)               |
|                                |                                    |
| > ag-1 impl opus  [active]    |    ag-1 ag-2 ag-3 ag-4 ag-5       |
|   ag-2 arch snnt  [active]    | ag-1  .    x    x    .    .       |
|   ag-3 res  opus  [idle]      | ag-2  x    .    .    x    .       |
|   ag-4 audit hai  [active]    | ag-3  x    .    .    .    x       |
|   ag-5 strat snnt [idle]      | ag-4  .    x    .    .    .       |
|                                | ag-5  .    .    x    .    .       |
| -------                       |                                    |
| ag-1 detail:                   | x = connected  . = no edge       |
| Task: T-04 implement          |                                    |
| Tokens: 2.1k/m  Cost: $1.23/h |                                    |
| Gate: 92%  Ctx: 67%           |                                    |
+----------------------------------------------------------------------+
```

##### Data sources

- Agent list: `GET /api/agents` **[existing]** via aggregator (poll 10s)
- Agent topology: `GET /api/agents/topology` **[existing]** via aggregator (poll 30s)
- Agent stats: `GET /api/agents/{id}/stats` **[existing]** via aggregator
- Real-time presence: WS `/ws` -- `presence_update` events
- For network view: Nexus `global` room `heartbeat_aggregate`

##### Component hierarchy

```
- AgentNetworkPage
  - ViewToggle (force-directed, grid, list)
  - ForceDirectedGraph (d3-force or similar, nodes = agents, edges = connections)
  - GridView (agents arranged in a grid by domain)
  - ListView (same as LiveAgents table)
  - ColorBySelector (domain, tier, model, status)
  - SizeBySelector (token burn, cost, episode count)
  - SelectedAgentDetail (bottom panel, shows stats for clicked agent)
```

##### State

- `agents: Array<AgentNode>` -- from topology endpoint
- `edges: Array<{ source, target }>` -- from topology endpoint
- `viewMode: "force" | "grid" | "list"`
- `colorBy: "domain" | "tier" | "model" | "status"`
- `sizeBy: "token_burn" | "cost" | "episodes"`
- `selectedAgent: string | null`

##### Interactions

- Click agent node -> select, show detail panel
- Drag agent node -> reposition in force layout
- Mouse wheel -> zoom in/out
- Change view toggle -> switch rendering mode
- In TUI: `j/k` navigate roster, `Tab` switch focus between roster and topology

##### Loading / empty / error states

- Loading: empty canvas with "Loading agent topology..." text
- Empty: "No agents online. Start agents with `roko atelier start`."
- Error: banner above canvas with retry

---

#### 7.4.2 Pheromone field

**Purpose:** Visualize the stigmergic signals (pheromones) that agents leave for each other -- knowledge trails, recommendations, warnings.
**TUI mapping:** F3 Agents sub-view (braille density map). See `widgets/braille.rs` for rendering primitives.
**Dashboard route:** `/network/pheromones`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Network]  Pheromone Field                                        |
+----------------------------------------------------------------------+
|                                                                      |
|  +------+  trail  +------+  trail  +------+                          |
|  | ag-1 | ------> | T-04 | ------> | ag-3 |                         |
|  +------+  [0.87] +------+  [0.65] +------+                         |
|      |                                 |                              |
|      | warning                         | recommendation              |
|      v                                 v                              |
|  +------+         +------+         +------+                          |
|  | gate4 |         | lib-x |        | playbook-23 |                  |
|  +------+         +------+         +------+                          |
|                                                                      |
|  Pheromone types: [All v]  Decay: [Fresh only v]  Min strength: 0.3  |
+----------------------------------------------------------------------+
| Selected: trail ag-1 -> T-04                                         |
| Strength: 0.87 | Age: 12m | Kind: implementation_path                |
| Content: "Used VCG auction for context assembly, reduced token..."   |
+----------------------------------------------------------------------+
```

##### Data sources

- Knowledge entries: `GET /api/knowledge/entries` **[existing]** via aggregator (poll 30s)
- Knowledge edges: `GET /api/knowledge/edges` **[existing]** via aggregator (poll 30s)
- Knowledge search: `GET /api/knowledge/search?q=...` **[existing]** via aggregator

##### Component hierarchy

```
- PheromoneFieldPage
  - PheromoneGraph (nodes = agents/tasks/artifacts, edges = pheromone trails)
  - FilterBar (pheromone type, decay filter, min strength slider)
  - PheromoneDetail (selected trail: strength, age, kind, content)
```

##### State

- `entries: Array<KnowledgeEntry>` -- from knowledge/entries
- `edges: Array<KnowledgeEdge>` -- from knowledge/edges
- `filterType: "all" | "trail" | "warning" | "recommendation"`
- `decayFilter: "all" | "fresh" | "recent"` -- based on Ebbinghaus decay
- `minStrength: number` -- 0.0-1.0 slider
- `selectedEdge: KnowledgeEdge | null`

##### Interactions

- Click edge -> show detail panel
- Adjust filters -> re-render graph
- Hover node -> highlight connected edges
- Search box -> `GET /api/knowledge/search`

##### Loading / empty / error states

- Loading: empty graph with "Loading pheromone data..."
- Empty: "No pheromone trails yet. Trails form as agents complete tasks and share knowledge."
- Error: banner with retry

##### TUI

TUI: F3 Agents sub-view (braille density map). See `widgets/braille.rs` for rendering primitives.

---

#### 7.4.3 Knowledge graph

**Purpose:** Browse the knowledge store -- engrams organized by kind, with relationships and decay visualization.
**TUI mapping:** F7 Inspect sub-view 4 (Knowledge Browser). See Task 3.3 in IMPL-10.
**Dashboard route:** `/network/knowledge`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Network]  Knowledge Graph                                        |
+----------------------------------------------------------------------+
| +----------------------------+  +---------------------------------+  |
| | Kinds                      |  | Entries (playbook)              |  |
| |                            |  |                                 |  |
| | playbook        (23)      |  | "Gate rung 4 recovery"          |  |
| | insight          (7)      |  |   Freshness: 94%  ||||||||..    |  |
| | episode_summary (142)     |  |   Source: ag-3, T-09             |  |
| | gate_result     (312)     |  |   Created: 2h ago                |  |
| | task_outcome     (47)     |  |                                 |  |
| | model_decision  (847)     |  | "VCG auction tuning"            |  |
| |                            |  |   Freshness: 87%  ||||||||.    |  |
| +----------------------------+  |   Source: ag-1, T-04             |  |
|                                 |   Created: 4h ago                |  |
| Search: [__________________]   |                                 |  |
|                                 | "HDC fingerprint matching"      |  |
|                                 |   Freshness: 45%  |||||.....   |  |
|                                 |   Source: ag-2, T-12             |  |
|                                 |   Created: 3d ago                |  |
|                                 +---------------------------------+  |
+----------------------------------------------------------------------+
| Selected: "Gate rung 4 recovery"                                     |
+----------------------------------------------------------------------+
| Content: When rung 4 (clippy) fails on lifetime errors, the         |
| most effective recovery is to...                                     |
+----------------------------------------------------------------------+
```

##### Data sources

- Knowledge kinds: `GET /api/knowledge/kinds` **[existing]** via aggregator
- Knowledge entries: `GET /api/knowledge/entries` **[existing]** via aggregator
- Knowledge search: `GET /api/knowledge/search?q=...` **[existing]** via aggregator
- Knowledge edges: `GET /api/knowledge/edges` **[existing]** via aggregator

##### Component hierarchy

```
- KnowledgeGraphPage
  - KindList
    - KindItem (name, count)
  - EntryList (filtered by selected kind)
    - EntryCard (title, freshness bar, source, age)
  - SearchBar
  - EntryDetail (full content, relationships, metadata)
```

##### State

- `kinds: Array<{ name, count }>`
- `selectedKind: string | null`
- `entries: Array<KnowledgeEntry>` -- filtered by kind
- `selectedEntry: KnowledgeEntry | null`
- `searchQuery: string`

##### Interactions

- Click kind -> filter entries to that kind
- Click entry -> show full detail
- Type in search -> debounce 300ms -> `GET /api/knowledge/search`
- Freshness bar shows Ebbinghaus decay visualization (layer 0 metaphor)
- Click freshness bar -> show exact decay parameters (layer 2)

##### Loading / empty / error states

- Loading: kind list skeleton, entry list skeleton
- Empty: "No knowledge entries. Knowledge accumulates as agents complete tasks."
- Error: per-section inline error

##### TUI

TUI: F7 Inspect sub-view 4 (Knowledge Browser). See Task 3.3 in IMPL-10.

---

#### 7.4.4 Swarm

**Purpose:** Show the global network of agents across all Nexus-connected instances. This is the "birds-eye view" of the entire Nunchi network.
**TUI mapping:** Not yet in TUI. Add as F3 sub-tab or new F9 tab.
**Dashboard route:** `/network/swarm`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Network]  Swarm                                                  |
+----------------------------------------------------------------------+
|                                                                      |
|  +-------------------+  +-------------------+  +------------------+  |
|  | Network size      |  | Total burn rate   |  | Network ISFR     |  |
|  | 47 agents         |  | $14.32/hr         |  | 1.032            |  |
|  | 12 domains        |  | [sparkline]       |  | [sparkline]      |  |
|  +-------------------+  +-------------------+  +------------------+  |
|                                                                      |
|  +----------------------------------------------------------------+  |
|  | Domain breakdown                                               |  |
|  |                                                                |  |
|  | trading    (18) [==================                ] $6.12/hr  |  |
|  | research   (12) [============                      ] $4.21/hr  |  |
|  | infra       (9) [=========                         ] $2.34/hr  |  |
|  | general     (8) [========                          ] $1.65/hr  |  |
|  +----------------------------------------------------------------+  |
|                                                                      |
|  +----------------------------------------------------------------+  |
|  | Agent heatmap (by domain x status)                             |  |
|  |          working  idle  sleeping  error                        |  |
|  | trading    12      4      1        1                           |  |
|  | research    8      3      1        0                           |  |
|  | infra       6      2      1        0                           |  |
|  | general     5      2      1        0                           |  |
|  +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- Nexus `global` room `heartbeat_aggregate` (WS, every 10s)
- Fallback: `GET /api/status` **[existing]** (poll 30s)
- ISFR: mirage-rs chain API (forwarded through roko-serve or Nexus)

##### Component hierarchy

```
- SwarmPage
  - SummaryCards
    - NetworkSizeCard (agents, domains)
    - BurnRateCard (total, sparkline)
    - IsfrCard (value, sparkline)
  - DomainBreakdown
    - DomainBar (name, agent_count, bar, burn_rate)
  - AgentHeatmap (domain x status matrix with cell counts)
```

##### State

- `aggregate: HeartbeatAggregate` -- from Nexus or status endpoint
- `burnHistory: Array<{ timestamp, rate }>` -- for sparkline
- `isfrHistory: Array<{ timestamp, value }>` -- for sparkline

##### Interactions

- Click domain in breakdown -> navigate to `/network/topology` filtered by that domain
- Hover heatmap cell -> tooltip with agent list

##### Loading / empty / error states

- Loading: card skeletons
- Empty: "No agents connected to the network."
- Error: "Cannot reach Nexus. Showing cached data."

---

### 7.5 Marketplace and jobs

#### 7.5.1 Job board

**Purpose:** Browse available bounties from BountyMarket.sol. Filter by type, reward, and required tier.
**TUI mapping:** F8 Marketplace tab. See Task 3.4 in IMPL-10.
**Dashboard route:** `/marketplace`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Marketplace]  Job Board                    [Filter v] [Sort v]   |
+----------------------------------------------------------------------+
|                                                                      |
| +----------------------------------------------------------------+  |
| |  #1042 | oracle-update | 500 DAEJI | Trusted+ | 2h deadline   |  |
| |  Update ETH/USD oracle from 3 sources, validate deviation <1%  |  |
| |  Posted: 45m ago | Bids: 3 | [View] [Apply]                    |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| |  #1041 | perps-liquidate | 2000 DAEJI | Elite | 30m deadline  |  |
| |  Monitor PERP position #8821, execute liquidation if margin <5%|  |
| |  Posted: 2h ago | Assigned: ag-7 | [View]                      |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| |  #1040 | research | 800 DAEJI | Standard+ | 24h deadline      |  |
| |  Research ERC-7702 integration patterns for account abstraction |  |
| |  Posted: 6h ago | Bids: 7 | [View] [Apply]                    |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| Showing 3 of 42 open jobs | Page 1 of 5 | [<] [>]                   |
+----------------------------------------------------------------------+
```

##### Data sources

- Job list: `GET /api/jobs` **[new -- needs jobs.rs in roko-serve]**
- Job types: `GET /api/jobs/types` **[new]** -- from JobTypeRegistry contract
- Worker registry: `GET /api/jobs/workers` **[new]** -- from WorkerRegistry contract
- Job events: Nexus `jobs` room (new job, bid, assignment, completion)
- Chain data: mirage-rs API for BountyMarket.sol reads

##### Component hierarchy

```
- JobBoardPage
  - FilterBar
    - JobTypeFilter (dropdown: all, oracle-update, perps-liquidate, research, etc.)
    - TierFilter (dropdown: all, Probation, Standard, Trusted, Elite)
    - RewardRange (min/max DAEJI)
    - DeadlineFilter (dropdown: <1h, <24h, <7d, all)
  - JobList
    - JobCard (id, type, reward, tier_required, deadline, description, bid_count)
  - Pagination
```

##### State

- `jobs: Array<Job>` -- from jobs endpoint
- `filters: { type, tier, rewardMin, rewardMax, deadline }`
- `sortBy: "reward" | "deadline" | "posted" | "bids"`
- `page: number`

##### Interactions

- Click "View" -> navigate to `/marketplace/jobs/:id`
- Click "Apply" -> opens bid form (requires wallet connection)
- Adjust filters -> re-fetch jobs
- Real-time: new jobs appear at top with highlight animation

##### Loading / empty / error states

- Loading: job card skeletons (3 placeholder cards)
- Empty: "No open jobs matching your filters."
- Error: banner with retry

##### TUI

TUI: F8 Marketplace tab. See Task 3.4 in IMPL-10.

---

#### 7.5.2 Create job

**Purpose:** Post a new bounty to BountyMarket.sol.
**TUI mapping:** Modal within F8 Marketplace tab, or CLI `roko job create`. See Task 3.4 in IMPL-10.
**Dashboard route:** `/marketplace/create`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Marketplace]  Create Job                                         |
+----------------------------------------------------------------------+
|                                                                      |
|  Job type: [oracle-update           v]                               |
|                                                                      |
|  Title: [Update ETH/USD oracle________________]                      |
|                                                                      |
|  Description:                                                        |
|  +--------------------------------------------------------------+   |
|  | Update the ETH/USD price oracle from at least 3 independent  |   |
|  | sources. Validate that deviation from median is less than 1%. |   |
|  | Submit the update transaction with gas price < 50 gwei.       |   |
|  +--------------------------------------------------------------+   |
|                                                                      |
|  Reward: [500__] DAEJI                                               |
|  Required tier: [Trusted          v]                                 |
|  Deadline: [2] hours                                                 |
|  Validator committee: [3]-of-[5]                                     |
|                                                                      |
|  Estimated escrow: 500 DAEJI + 25 DAEJI (validator fee)             |
|                                                                      |
|  [Cancel]                                    [Post Job - 525 DAEJI]  |
+----------------------------------------------------------------------+
```

##### Data sources

- Job types: `GET /api/jobs/types` **[new]** -- for dropdown
- Post job: `POST /api/jobs` **[new]** -- creates on-chain via BountyMarket.sol
- Wallet balance: Privy SDK -> on-chain DAEJI balance

##### Component hierarchy

```
- CreateJobPage
  - JobTypeSelector (dropdown)
  - TitleInput
  - DescriptionTextarea
  - RewardInput (numeric + DAEJI label)
  - TierSelector (dropdown)
  - DeadlineInput (number + unit dropdown)
  - ValidatorConfig (n-of-m inputs)
  - EscrowSummary (computed: reward + validator fee)
  - ActionButtons (Cancel, Post Job)
```

##### State

- `form: { type, title, description, reward, tier, deadline, validators }`
- `escrowTotal: number` -- computed from reward + validator fee
- `walletBalance: number` -- from Privy
- `isSubmitting: boolean`

##### Interactions

- Fill form -> escrow total updates live
- Click "Post Job" -> wallet signature required -> `POST /api/jobs` -> on-chain transaction -> navigate to job detail on success
- Insufficient balance -> "Post Job" button disabled with tooltip

##### Loading / empty / error states

- Loading: spinner on submit button
- Error: inline errors below each invalid field, transaction error in banner

##### TUI

TUI: Modal within F8 Marketplace tab, or CLI `roko job create`. See Task 3.4 in IMPL-10.

---

#### 7.5.3 Job detail

**Purpose:** View a specific job's full details, bids, assignment, and resolution status.
**TUI mapping:** Modal within F8 Marketplace tab (`render_job_detail_modal`). See Task 3.4 in IMPL-10.
**Dashboard route:** `/marketplace/jobs/:id`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Job Board]  Job #1042: Update ETH/USD oracle                     |
+----------------------------------------------------------------------+
| +--------------------------------+  +-----------------------------+  |
| | Details                        |  | Status                      |  |
| |                                |  |                             |  |
| | Type: oracle-update            |  | [Posted] -> [Assigned] ->   |  |
| | Reward: 500 DAEJI              |  | [Submitted] -> [Resolved]   |  |
| | Required tier: Trusted+        |  |      ^                      |  |
| | Deadline: 2h (1h 15m left)     |  |      current                |  |
| | Poster: 0x1234...5678          |  |                             |  |
| | Posted: 45m ago                |  | Assigned to: ag-7           |  |
| |                                |  | Assigned: 30m ago           |  |
| +--------------------------------+  +-----------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Description                                                     |  |
| | Update the ETH/USD price oracle from at least 3 independent    |  |
| | sources. Validate deviation from median < 1%. Submit update     |  |
| | transaction with gas price < 50 gwei.                           |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Bids (3)                                                       |  |
| |                                                                |  |
| | ag-7   | Trusted | Rep: 0.94 | Stake: 2500 | ETA: 45m  [asgn]|  |
| | ag-12  | Trusted | Rep: 0.88 | Stake: 1800 | ETA: 60m        |  |
| | ag-3   | Elite   | Rep: 0.96 | Stake: 5000 | ETA: 30m        |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Validation (pending)                                            |  |
| | Committee: 3-of-5 | Votes: 0/3 required                       |  |
| +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- Job detail: `GET /api/jobs/{id}` **[new]**
- Job bids: `GET /api/jobs/{id}/bids` **[new]**
- Job validation: `GET /api/jobs/{id}/validation` **[new]**
- Real-time updates: Nexus `jobs` room events (bid, assignment, submission, resolution)

##### Component hierarchy

```
- JobDetailPage
  - JobHeader (id, title)
  - DetailsCard (type, reward, tier, deadline, poster)
  - StatusTimeline (posted -> assigned -> submitted -> resolved)
  - DescriptionCard
  - BidList
    - BidRow (agent, tier, reputation, stake, eta, assigned badge)
  - ValidationCard (committee config, vote count, vote results)
```

##### State

- `job: JobDetail` -- from job detail endpoint
- `bids: Array<Bid>` -- from bids endpoint
- `validation: ValidationState` -- from validation endpoint

##### Interactions

- Click agent ID in bid list -> navigate to agent detail
- If user is the poster: "Accept bid" button on each bid row -> on-chain assignment
- If user is the assigned agent: "Submit work" button -> on-chain submission
- If user is a validator: "Vote" buttons (accept/reject) -> on-chain vote

##### Loading / empty / error states

- Loading: card skeletons
- Error: full-page error with "Back to Job Board" link

##### TUI

TUI: Modal within F8 Marketplace tab (`render_job_detail_modal`). See Task 3.4 in IMPL-10.

---

### 7.6 Agent Studio

#### 7.6.1 Agent overview

**Purpose:** Dashboard for a specific agent -- its configuration, performance history, episodes, and skills.
**TUI mapping:** F3 Agents right panel when an agent is selected. Already implemented in `agents_view.rs`.
**Dashboard route:** `/agent-studio/:id/overview`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Agent Studio]  Overview                  [Agent: implementer v]  |
+----------------------------------------------------------------------+
| +---------------------+  +-----------+  +-----------+  +-----------+ |
| | Episodes            |  | Gate rate |  | Cost      |  | Tier      | |
| | 142 total           |  | 87%       |  | $18.42    |  | T1/Theta  | |
| | 12 today            |  | [spark]   |  | [spark]   |  |           | |
| +---------------------+  +-----------+  +-----------+  +-----------+ |
|                                                                      |
| +--------------------------------+  +-----------------------------+  |
| | Recent episodes                |  | Skills                      |  |
| |                                |  |                             |  |
| | ep-142 T-04 implement  [pass] |  | * implement_feature         |  |
| |   4,200 tokens | $0.52        |  | * fix_bug                   |  |
| | ep-141 T-04 implement  [fail] |  | * refactor_module           |  |
| |   rung 4 clippy failure       |  | * write_test                |  |
| | ep-140 T-03 plan-gen   [pass] |  | * code_review               |  |
| |   1,800 tokens | $0.22        |  |                             |  |
| +--------------------------------+  +-----------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Performance over time (7d)                                      |  |
| |                                                                |  |
| | Gate pass rate:  [line chart over 7 days]                      |  |
| | Token burn:      [line chart over 7 days]                      |  |
| | Cost per task:   [line chart over 7 days]                      |  |
| +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- Agent info: `GET /api/agents/{id}` **[existing]** via aggregator
- Agent stats: `GET /api/agents/{id}/stats` **[existing]** via aggregator
- Agent skills: `GET /api/agents/{id}/skills` **[existing]** via aggregator
- Agent episodes: `GET /api/agents/{id}/episodes` **[existing]**
- Metrics: `GET /api/metrics/c_factor` **[existing]**, `GET /api/metrics/gate_rate` **[existing]**

##### Component hierarchy

```
- AgentOverviewPage
  - AgentSelector (dropdown)
  - SummaryCards
    - EpisodesCard (total, today)
    - GateRateCard (percentage, sparkline)
    - CostCard (total, sparkline)
    - TierCard (cognitive tier)
  - RecentEpisodes
    - EpisodeRow (id, task, result_badge, tokens, cost)
  - SkillsList
    - SkillChip (name)
  - PerformanceCharts
    - GatePassRateChart (7d line chart)
    - TokenBurnChart (7d line chart)
    - CostPerTaskChart (7d line chart)
```

##### State

- `selectedAgentId: string`
- `agentInfo: AgentInfo`
- `agentStats: AgentStats`
- `episodes: Array<Episode>`
- `skills: Array<string>`

##### Interactions

- Select agent -> reload all data for that agent
- Click episode -> expand to show full episode detail (gate results, output, timing)
- Click skill -> filter episodes to tasks that used that skill

##### Loading / empty / error states

- Loading: card skeletons, episode list skeleton
- Empty: "Select an agent to view its overview."
- Error: per-section inline error

##### Mock data to remove

- Remove mock `agents` array from current AgentOverviewPanel
- Remove "View Full Audit Trail" and "View All Insights" dead buttons

##### TUI

TUI: F3 Agents right panel when agent is selected. Already implemented in `agents_view.rs`.

---

#### 7.6.2 Strategy

**Purpose:** Configure an agent's behavior -- skills, context assembly settings, model routing preferences.
**TUI mapping:** F6 Config (agent-specific config sections). Already partially implemented.
**Dashboard route:** `/agent-studio/:id/strategy`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Agent Studio]  Strategy                  [Agent: implementer v]  |
+----------------------------------------------------------------------+
| +----------------------------------------------------------------+  |
| | Skills                                                          |  |
| | [x] implement_feature    [x] fix_bug                           |  |
| | [x] refactor_module      [ ] write_docs                        |  |
| | [x] write_test           [ ] deploy                            |  |
| | [x] code_review                                                |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +--------------------------------+  +-----------------------------+  |
| | Context assembly               |  | Model routing              |  |
| |                                |  |                             |  |
| | Max context tokens: [32000_]   |  | Preferred: [opus-4      v] |  |
| | Include playbooks: [Yes v]     |  | Fallback:  [sonnet-4   v] |  |
| | Include episodes:  [Last 5 v]  |  | Budget:    [$5.00/hr   _] |  |
| | VCG bidding: [Enabled v]       |  | Auto-route: [Enabled  v]  |  |
| +--------------------------------+  +-----------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Gate configuration                                              |  |
| |                                                                |  |
| | Max rung: [6 v]                                                |  |
| | Rung 4 (clippy): [-D warnings v]                               |  |
| | Rung 5 (test): [cargo test --workspace v]                      |  |
| | Auto-retry on failure: [Yes, up to 3 v]                        |  |
| +----------------------------------------------------------------+  |
|                                                                      |
|                                        [Reset to Defaults] [Save]   |
+----------------------------------------------------------------------+
```

##### Data sources

- Agent config: `GET /api/config` **[existing]** -- agent section
- Save config: `PUT /api/config` **[existing]** -- partial merge
- Templates: `GET /api/templates` **[existing]** -- for available role templates

##### Component hierarchy

```
- StrategyPage
  - AgentSelector
  - SkillsSection
    - SkillCheckbox (name, enabled)
  - ContextAssemblySection
    - NumberInput (max context tokens)
    - ToggleSelect (include playbooks, episodes, VCG)
  - ModelRoutingSection
    - ModelSelector (preferred, fallback)
    - NumberInput (budget)
    - ToggleSelect (auto-route)
  - GateConfigSection
    - RungSelector (max rung)
    - RungConfig (per-rung command/flags)
    - RetryConfig
  - ActionButtons (Reset, Save)
```

##### State

- `config: AgentConfig` -- loaded from config endpoint
- `pendingChanges: Partial<AgentConfig>` -- unsaved edits
- `isDirty: boolean` -- computed from pendingChanges
- `isSaving: boolean`

##### Interactions

- Change any field -> set pendingChanges, enable Save button
- Click Save -> `PUT /api/config` with pendingChanges -> success notification
- Click Reset -> clear pendingChanges
- Unsaved changes + navigate away -> confirmation dialog

##### Loading / empty / error states

- Loading: form skeleton
- Error: inline per-field errors on validation, banner on save failure

##### Mock data to remove

- StrategyPanel: all config except skills is local state lost on reload -- wire to `PUT /api/config`

##### TUI

TUI: F6 Config (agent-specific config sections). Already partially implemented.

---

#### 7.6.3 Keys

**Purpose:** Manage API keys for agent-to-service authentication.
**TUI mapping:** Not in TUI. Use `roko config` CLI for now.
**Dashboard route:** `/agent-studio/:id/keys`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Agent Studio]  API Keys                                          |
+----------------------------------------------------------------------+
|                                                                      |
| +----------------------------------------------------------------+  |
| | Active keys                                                    |  |
| |                                                                |  |
| | roko_...4f2a | Label: "dashboard" | Created: 3d ago | [Revoke]|  |
| | roko_...8b1c | Label: "ci"        | Created: 7d ago | [Revoke]|  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Create new key                                                 |  |
| |                                                                |  |
| | Label: [________________]                                      |  |
| | Scope: [read-write      v]                                     |  |
| |                                                                |  |
| | [Create Key]                                                   |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Key created successfully!                                      |  |
| |                                                                |  |
| | roko_sk_live_REDACTED                |  |
| |                                              [Copy]            |  |
| |                                                                |  |
| | Save this key. You will not see it again.                      |  |
| +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- List keys: `GET /api/keys` **[new]**
- Create key: `POST /api/keys` **[new]**
- Revoke key: `DELETE /api/keys/{id}` **[new]**

##### Component hierarchy

```
- KeysPage
  - KeyList
    - KeyRow (redacted key, label, created, revoke button)
  - CreateKeyForm
    - LabelInput
    - ScopeSelector (read-only, read-write, admin)
    - CreateButton
  - KeyRevealCard (shown once after creation, with copy button)
```

##### State

- `keys: Array<{ id, prefix, label, scope, created_at }>`
- `newKey: string | null` -- shown once after creation
- `isCreating: boolean`

##### Interactions

- Click "Create Key" -> `POST /api/keys` -> reveal new key in KeyRevealCard
- Click "Copy" -> clipboard API
- Click "Revoke" -> confirmation dialog -> `DELETE /api/keys/{id}`

##### Loading / empty / error states

- Loading: list skeleton
- Empty: "No API keys. Create one to authenticate CLI or external services."
- Error: inline on create/revoke failure

##### Mock data to remove

- KeysPanel is entirely static (61 lines, no API calls at all). Replace completely.

---

#### 7.6.4 Deploy

**Purpose:** Deploy agent configurations to cloud infrastructure.
**TUI mapping:** Not in TUI. Use `roko deploy` CLI.
**Dashboard route:** `/agent-studio/:id/deploy`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Agent Studio]  Deploy                                            |
+----------------------------------------------------------------------+
|                                                                      |
| +----------------------------------------------------------------+  |
| | Active deployments                                              |  |
| |                                                                |  |
| | deploy-001 | railway | template: researcher | Status: running  |  |
| |   Created: 2h ago | [Logs] [Teardown]                          |  |
| |                                                                |  |
| | deploy-002 | railway | template: implementer | Status: stopped |  |
| |   Created: 1d ago | [Logs] [Restart] [Teardown]                |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | New deployment                                                 |  |
| |                                                                |  |
| | Template: [researcher        v]                                |  |
| | Backend:  [railway-api       v]                                |  |
| | Parameters:                                                    |  |
| |   model: [opus-4___________]                                   |  |
| |   domain: [trading__________]                                  |  |
| |                                                                |  |
| | [Deploy]                                                       |  |
| +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- List deployments: `GET /api/deployments` **[existing]**
- Create deployment: `POST /api/deployments` **[existing]**
- Get deployment: `GET /api/deployments/{id}` **[existing]**
- Teardown: `DELETE /api/deployments/{id}` **[existing]**
- Logs: `GET /api/deployments/{id}/logs` **[existing]**
- Templates: `GET /api/templates` **[existing]**

##### Component hierarchy

```
- DeployPage
  - DeploymentList
    - DeploymentCard (id, backend, template, status, actions)
  - CreateDeploymentForm
    - TemplateSelector
    - BackendSelector
    - ParameterInputs (dynamic based on template)
    - DeployButton
```

##### State

- `deployments: Array<Deployment>`
- `templates: Array<Template>`
- `form: { template, backend, params }`
- `isDeploying: boolean`

##### Interactions

- Click "Deploy" -> `POST /api/deployments` -> track via WS events
- Click "Logs" -> navigate to deployment log view
- Click "Teardown" -> confirmation -> `DELETE /api/deployments/{id}`

##### Loading / empty / error states

- Loading: deployment list skeleton
- Empty: "No active deployments. Deploy an agent template to get started."
- Error: banner on deploy failure with error details

##### Mock data to remove

- Remove hardcoded binding code "847291"
- Remove likely-404 install URL

---

### 7.7 Atelier (workspace)

#### 7.7.1 Workspace dashboard

**Purpose:** Overview of the current Atelier -- its plans, agents, cost, and progress.
**TUI mapping:** F1 Dashboard (primary view)
**Dashboard route:** `/atelier`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Atelier]  feature-x                [Cruise] [2h 14m] [$12.47]   |
+----------------------------------------------------------------------+
| +---------------------+  +-----------+  +-----------+  +-----------+ |
| | Plans               |  | Agents    |  | Gate rate |  | Episodes  | |
| | 3 total, 1 active   |  | 5 running |  | 87%       |  | 142       | |
| | [===========>   ] 67|  | 2 idle    |  |           |  | 12 today  | |
| +---------------------+  +-----------+  +-----------+  +-----------+ |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Active plan: feature-x                                          |  |
| |                                                                |  |
| | T-01 draft-prd      [done]   ag-1 impl  2m                    |  |
| | T-02 research        [done]   ag-3 res  14m                    |  |
| | T-03 plan-gen        [done]   ag-1 impl  5m                    |  |
| | T-04 implement     > [active] ag-1 impl  running...            |  |
| | T-05 gate            [pend]   --                                |  |
| | T-06 review          [pend]   --                                |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +--------------------------------+  +-----------------------------+  |
| | Recent output                  |  | Gate results                |  |
| |                                |  |                             |  |
| | [ag-1] Implementing T-04...   |  | T-03 rung 6 [pass] 1m ago  |  |
| | Modified: src/widget.rs        |  | T-02 rung 6 [pass] 15m ago |  |
| | Added: tests/widget_test.rs    |  | T-01 rung 4 [pass] 20m ago |  |
| +--------------------------------+  +-----------------------------+  |
+----------------------------------------------------------------------+
```

##### Wireframe (TUI -- F1 Dashboard)

```
+----------------------------------------------------------------------+
| F1 Dash | F2 Plans | F3 Agents | F4 Git | F5 Logs | F6 Cfg | F7 Ins |
+-------- a:Agents  o:Out  d:Diff  e:Gate  g:Git  m:MCP  L:Learn  P:--+
+-------------------------------+--------------------------------------+
| PLAN TREE                     | SUB-TAB CONTENT                      |
|                               | (varies by selected sub-tab)         |
| v feature-x [=====>   ] 67%  |                                      |
|   T-01 draft-prd      [done] | [a:Agents] Agent roster + detail     |
|   T-02 research        [done]| [o:Output] Live agent stdout         |
|   T-03 plan-gen        [done]| [d:Diff]   Git diff for active task  |
|   T-04 implement     [active]| [e:Gate]   Gate results table        |
|   T-05 gate           [pend] | [g:Git]    Git status + recent cmts  |
|   T-06 review          [pend]| [m:MCP]    MCP server status         |
|                               | [L:Learn]  Learning metrics          |
|                               | [P:Procs]  Process supervisor        |
+-------------------------------+--------------------------------------+
| WAVE [==============>         ] 67%  |  tok/m: 7.3k  | $4.83/hr    |
+----------------------------------------------------------------------+
```

##### Data sources

- Status: `GET /api/status` **[existing]** (poll 10s)
- Plans: `GET /api/plans` **[existing]** (poll 15s)
- Active plan tasks: `GET /api/plans/{id}` **[existing]** (poll 5s while active)
- Agents: `GET /api/managed-agents` **[existing]** (poll 10s)
- Episodes: `GET /api/episodes` **[existing]** (poll 15s)
- Gate results: `GET /api/metrics/gate_rate` **[existing]**
- Real-time: WS `/ws` -- all event types **[existing]**
- Cost: `GET /api/metrics/summary` **[existing]**

##### Component hierarchy

```
- AtelierDashboard
  - AtelierHeader (name, regime badge, duration, cost)
  - SummaryCards
    - PlansCard (count, active, progress bar)
    - AgentsCard (running, idle)
    - GateRateCard (percentage)
    - EpisodesCard (total, today)
  - ActivePlanSection
    - TaskList
      - TaskRow (id, description, status, agent, duration)
  - BottomPanels (two-column)
    - RecentOutput (scrolling agent output)
    - GateResults (most recent gate pass/fail list)
```

##### State

- `atelierName: string` -- from status or CLI context
- `plans: Array<PlanSummary>`
- `activePlan: PlanDetail | null`
- `agents: Array<ManagedAgent>`
- `regime: string`
- `costTotal: number`
- `duration: number` -- computed from atelier start time

##### Interactions

- Click task -> task detail modal (description, gate results, output)
- Click agent -> navigate to Observatory/Agents with that agent selected
- Click plan -> navigate to Observatory/Plans with that plan selected
- In TUI: `a/o/d/e/g/m/L/P` keys switch sub-tabs in right panel

##### Loading / empty / error states

- Loading: card skeletons + empty task list
- Empty: "No active atelier. Start one with `roko atelier start --name <name>`."
- Error: banner: "Lost connection to roko-serve. Reconnecting..."

---

#### 7.7.2 PRD browser

**Purpose:** Browse, create, and manage PRDs within the current atelier.
**TUI mapping:** F9 Atelier tab right panel. See Task 3.5 in IMPL-10.
**Dashboard route:** `/atelier/prds`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Atelier]  PRDs                          [+ New Idea] [+ Draft]  |
+----------------------------------------------------------------------+
| +----------------------------+  +---------------------------------+  |
| | PRDs                       |  | system-prompt-wiring            |  |
| |                            |  |                                 |  |
| | * system-prompt-wiring     |  | Status: published               |  |
| |   [published]              |  | Created: 2d ago                 |  |
| | * dashboard-redesign       |  | Plans: 2 generated              |  |
| |   [draft]                  |  |                                 |  |
| | * gate-improvements        |  | # System prompt wiring          |  |
| |   [idea]                   |  |                                 |  |
| |                            |  | Wire the SystemPromptBuilder    |  |
| +----------------------------+  | into orchestrate.rs so that...  |  |
|                                 |                                 |  |
| Coverage:                       |                                 |  |
| PRDs: 3 | Plans: 2 | Tasks: 12 | Done: 8 (67%)                   |  |
|                                 +---------------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- PRD list: `GET /api/prds` **[existing]** (poll 30s)
- PRD detail: `GET /api/prds/{slug}` **[existing]**
- PRD coverage: `GET /api/prds/status` **[existing]**
- Create idea: `POST /api/prds/ideas` **[existing]**
- Draft PRD: `POST /api/prds/{slug}/draft` **[existing]**
- Promote: `POST /api/prds/{slug}/promote` **[existing]**
- Generate plan: `POST /api/prds/{slug}/plan` **[existing]**

##### Component hierarchy

```
- PrdBrowserPage
  - PrdList
    - PrdItem (slug, status badge)
  - PrdDetail
    - PrdHeader (title, status, created, plan count)
    - PrdContent (markdown renderer)
    - ActionBar (Promote, Generate Plan, Enhance with Research)
  - CoverageBar (PRDs, Plans, Tasks, Done percentage)
  - NewIdeaModal
  - NewDraftModal
```

##### State

- `prds: Array<PrdSummary>`
- `selectedSlug: string | null`
- `prdContent: string` -- markdown content of selected PRD
- `coverage: { prds, plans, tasks, done }`

##### Interactions

- Click PRD in list -> load detail
- Click "Promote" -> `POST /api/prds/{slug}/promote` -> refresh
- Click "Generate Plan" -> `POST /api/prds/{slug}/plan` -> track operation
- Click "+ New Idea" -> modal with text input -> `POST /api/prds/ideas`
- Click "+ Draft" -> modal with title input -> `POST /api/prds/{slug}/draft`

##### Loading / empty / error states

- Loading: list skeleton, detail skeleton
- Empty: "No PRDs. Capture an idea with `roko prd idea 'your idea'`."
- Error: inline on action failure

##### TUI

TUI: F9 Atelier tab right panel. See Task 3.5 in IMPL-10.

---

#### 7.7.3 Execution monitor

**Purpose:** Real-time view of plan execution -- task output streaming, gate results as they happen, agent activity.
**TUI mapping:** F1 Dashboard (the primary view). Already the main TUI experience.
**Dashboard route:** `/atelier/execution`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Atelier]  Execution Monitor              Plan: feature-x        |
+----------------------------------------------------------------------+
| +----------------------------------------------------------------+  |
| | T-04: implement                                    [active]     |  |
| | Agent: ag-1 (implementer, opus-4)                              |  |
| | Duration: 12m 34s | Tokens: 24,100 in / 18,200 out            |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +----------------------------------------------------------------+  |
| | Live output                                         [Auto-scroll]|  |
| |                                                                |  |
| | > Reading crates/roko-compose/src/system_prompt_builder.rs     |  |
| | > Found existing SystemPromptBuilder with 9 layers             |  |
| | > Modifying orchestrate.rs to import and use builder           |  |
| | > Added RoleSystemPromptSpec construction at line 847           |  |
| | > Running cargo check... OK                                    |  |
| | > Running cargo clippy... 1 warning (unused import)            |  |
| | > Fixing unused import at orchestrate.rs:12                    |  |
| | > Running cargo clippy... OK                                   |  |
| | > Running cargo test --workspace...                            |  |
| | _                                                              |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| +--------------------------------+  +-----------------------------+  |
| | Gate pipeline                  |  | Diff preview                |  |
| |                                |  |                             |  |
| | Rung 0 syntax      [pass]     |  | orchestrate.rs              |  |
| | Rung 1 cargo check  [pass]    |  | +use roko_compose::...      |  |
| | Rung 2 clippy        [pass]   |  | +let spec = RoleSys...      |  |
| | Rung 3 fmt           [pass]   |  |  ...                        |  |
| | Rung 4 clippy -D     [pass]   |  | tests/compose_test.rs       |  |
| | Rung 5 test          [running]|  | +#[test]                    |  |
| | Rung 6 review        [pend]   |  | +fn test_builder()...       |  |
| +--------------------------------+  +-----------------------------+  |
+----------------------------------------------------------------------+
```

##### Data sources

- Plan status: `GET /api/plans/{id}/status` **[existing]** (poll 5s)
- Agent output: WS `/ws` -- `AgentOutput` events **[existing]** (streaming)
- Gate results: WS `/ws` -- gate-related events **[existing]**
- Diff: agent output contains diff data, or fetch via `GET /api/agents/{id}/logs` **[existing]**

##### Component hierarchy

```
- ExecutionMonitorPage
  - TaskHeader (id, description, agent, duration, tokens)
  - LiveOutput (scrollable, auto-scroll toggle, ANSI color support)
  - BottomPanels (two-column)
    - GatePipeline
      - RungRow (rung number, name, status badge)
    - DiffPreview (truncated unified diff)
```

##### State

- `activePlanId: string`
- `activeTaskId: string`
- `outputLines: Array<string>` -- from WS AgentOutput events
- `gateResults: Array<{ rung, name, status }>` -- from WS events
- `diff: string` -- from agent output
- `autoScroll: boolean` -- default true

##### Interactions

- Output auto-scrolls unless user scrolls up manually (disable auto-scroll)
- Click gate result -> expand to show full output for that rung
- Click diff file name -> expand to show full diff for that file
- Toggle auto-scroll -> sticky to bottom when enabled

##### Loading / empty / error states

- Loading: "Waiting for agent output..."
- Empty: "No active execution. Start a plan to see live output."
- Error: "Connection lost. Attempting reconnect..."

##### TUI

TUI: F1 Dashboard (the primary view). Already the main TUI experience.

---

### 7.8 Settings

#### 7.8.1 Config editor

**Purpose:** View and edit the roko.toml configuration.
**TUI mapping:** F6 Config. Already implemented in `config_view.rs`.
**Dashboard route:** `/settings/config`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Settings]  Configuration                          [Save] [Reset] |
+----------------------------------------------------------------------+
|                                                                      |
| +----------------------------------------------------------------+  |
| | [Agent] [Gate] [Learning] [Serve] [MCP] [PRD]                 |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| Agent                                                                |
| +----------------------------------------------------------------+  |
| | model           | [opus-4_______________]                      |  |
| | fallback_model   | [sonnet-4_____________]                      |  |
| | max_tokens       | [32000___]                                   |  |
| | role             | [implementer         v]                      |  |
| | mcp_config       | [~/.config/mcp.json___]                      |  |
| +----------------------------------------------------------------+  |
|                                                                      |
| Gate                                                                 |
| +----------------------------------------------------------------+  |
| | max_rung         | [6 v]                                        |  |
| | auto_retry       | [true v]                                     |  |
| | retry_limit      | [3___]                                       |  |
| +----------------------------------------------------------------+  |
|                                                                      |
+----------------------------------------------------------------------+
```

##### Data sources

- Read config: `GET /api/config` **[existing]**
- Write config: `PUT /api/config` **[existing]**
- Reload config: `POST /api/config/reload` **[existing]**

##### Component hierarchy

```
- ConfigEditorPage
  - SectionTabs (Agent, Gate, Learning, Serve, MCP, PRD)
  - ConfigSection
    - ConfigField (key, current_value, input_type, description)
  - ActionBar (Save, Reset, Reload from disk)
```

##### State

- `config: RokoConfig` -- from config endpoint
- `pendingChanges: Partial<RokoConfig>`
- `activeSection: string`
- `isDirty: boolean`

##### Interactions

- Edit field -> mark dirty, enable Save
- Click Save -> `PUT /api/config` with pendingChanges
- Click Reset -> clear pendingChanges
- Click "Reload from disk" -> `POST /api/config/reload`
- In TUI: `j/k` navigate fields, `Enter` to edit, `Esc` to cancel

##### Loading / empty / error states

- Loading: form skeleton
- Error: inline on save failure, banner on load failure

##### TUI

TUI: F6 Config. Already implemented in `config_view.rs`.

---

#### 7.8.2 Theme (dashboard only)

**Purpose:** Customize dashboard appearance.
**Dashboard route:** `/settings/theme`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Settings]  Theme                                                 |
+----------------------------------------------------------------------+
|                                                                      |
|  Color scheme: [ROSEDUST v]  [Dark v]  [High Contrast]              |
|                                                                      |
|  Preview:                                                            |
|  +--------------------------------------------------------------+   |
|  | Sample card with current theme colors                         |   |
|  | Text: The quick brown fox...                                  |   |
|  | Accent: [=====>     ] 50%                                     |   |
|  | Status: [ok] [warn] [error]                                   |   |
|  +--------------------------------------------------------------+   |
|                                                                      |
|  Information density: [Normal v]  (Compact / Normal / Spacious)     |
|  Font size: [14px v]  (12 / 14 / 16)                                |
|  Animations: [Enabled v]                                             |
|                                                                      |
+----------------------------------------------------------------------+
```

##### Data sources

- All local (localStorage). No server interaction.

##### Component hierarchy

```
- ThemePage
  - ColorSchemeSelector (ROSEDUST and any future palettes)
  - VariantSelector (Dark / High Contrast)
  - ThemePreviewCard (live preview with sample text, badge, progress bar)
  - DensitySelector (Compact / Normal / Spacious)
  - FontSizeSelector (12 / 14 / 16)
  - AnimationsToggle (Enabled / Disabled)
```

##### State

- `theme: { colorScheme, variant, density, fontSize, animations }` -- localStorage

##### Interactions

- Click theme card -> apply immediately. Theme persists to localStorage. No save button needed.
- Change density or font size -> preview updates instantly. Change persists to localStorage.

##### Loading / empty / error states

- No loading state -- themes are instant.
- No error state -- all data is local preference.

##### TUI

TUI: Not applicable -- this feature is dashboard-only. CLI equivalent: `roko config set tui.theme <name>`.

##### Mock data tags

None -- all data is local preference.

---

#### 7.8.3 Notifications (dashboard only)

**Purpose:** Configure what events trigger browser notifications.
**Dashboard route:** `/settings/notifications`

##### Wireframe (dashboard)

```
+----------------------------------------------------------------------+
| [<- Settings]  Notifications                                         |
+----------------------------------------------------------------------+
|                                                                      |
|  Browser notifications: [Enabled v]                                  |
|                                                                      |
|  Notify on:                                                          |
|  [x] Gate failure                                                    |
|  [x] Plan completed                                                 |
|  [x] Agent error                                                     |
|  [ ] Agent idle                                                      |
|  [x] Cost threshold exceeded                                        |
|  [ ] New job posted (marketplace)                                    |
|                                                                      |
|  Cost threshold: [$__50.00] per day                                  |
|                                                                      |
+----------------------------------------------------------------------+
```

##### Data sources

- All local (localStorage). Notifications triggered by WS events via the global WS connection in `wsStore.ts`.

##### Component hierarchy

```
- NotificationsPage
  - NotificationsToggle (master enable/disable, requests Browser Notification API permission)
  - EventCheckboxList
    - EventCheckbox (label, enabled toggle) -- one per: gateFailure, planCompleted, agentError, agentIdle, costThreshold, newJob
  - CostThresholdInput (numeric, USD per day, only relevant when costThreshold is enabled)
```

##### State

- `notificationPrefs: { enabled, gateFailure, planCompleted, agentError, agentIdle, costThreshold, newJob, costThresholdUsd }` -- localStorage
- `permissionState: "default" | "granted" | "denied"` -- from `Notification.permission`

##### Interactions

- Toggle master enable -> request `Notification.permission` if enabling and permission is "default". Persist to localStorage.
- Toggle individual event -> update localStorage. No server call.
- Change cost threshold -> update localStorage on blur.
- When WS event fires matching an enabled preference -> `new Notification(...)` with event details.

##### Loading / empty / error states

- No loading state -- preferences load instantly from localStorage.
- Error state: "Notifications blocked by your browser. Update permissions in browser settings." (shown when `permissionState === "denied"`).

##### TUI

TUI: Not applicable -- this feature is dashboard-only. CLI equivalent: `roko status` for monitoring.

##### Mock data tags

None -- all data is local preference.

---

## 8. Widget and component catalog

Shared primitives used across both surfaces. Each widget has a dashboard (React) and TUI (ratatui) implementation.

### 8.1 Status badge

Renders a colored label for status values.

| Status | Dashboard | TUI |
|--------|-----------|-----|
| active | Green dot + "active" text | `[active]` in SAGE color |
| idle | Gray dot + "idle" text | `[idle]` in TEXT_DIM |
| done | Blue check + "done" text | `[done]` in DREAM |
| failed | Red X + "failed" text | `[FAIL]` in EMBER, bold |
| pending | Yellow clock + "pending" text | `[pend]` in WARNING |

### 8.2 Progress bar

Horizontal bar showing completion percentage.

**Dashboard:** Tailwind bar with percentage label and optional gradient.
**TUI:** Block characters (`\u2591` through `\u2588`) for sub-character precision. Already implemented in `plans_view.rs`.

### 8.3 Sparkline

Mini chart showing value history over time.

**Dashboard:** SVG path element, 60px wide, inline with text.
**TUI:** `ratatui::widgets::Sparkline` -- already used in `token_sparkline.rs`.

### 8.4 Regime badge

Shows the current operating regime with appropriate color.

| Regime | Dashboard | TUI |
|--------|-----------|-----|
| Cruise | Green pill with "Cruise" | `[cruise]` in SAGE |
| Volatile | Amber pill with "Volatile" | `[VOLATILE]` in WARNING, bold |
| Crisis | Red pill with "CRISIS" | `[CRISIS]` in EMBER, bold, blinking |

### 8.5 Cognitive tier indicator

Shows T0/T1/T2 with the brain-wave metaphor at layer 0.

**Dashboard:** Colored chip with tier label. Tooltip shows technical tier name.
**TUI:** `T0`/`T1`/`T2` with color. `gamma`/`theta`/`delta` label adjacent.

### 8.6 Context gauge

Horizontal bar showing context window utilization (0-100%).

**Dashboard:** Segmented bar (green < 70%, amber 70-90%, red > 90%).
**TUI:** Same as progress bar, color-coded by utilization level.

### 8.7 Cost display

Formatted currency display with optional sparkline.

**Dashboard:** "$12.47" with optional "/hr" suffix. Sparkline on hover.
**TUI:** "$12.47/hr" inline. Sparkline in separate widget.

### 8.8 Gate result row

Single gate rung result with status.

**Dashboard:** Row with rung number, name, pass/fail badge, duration.
**TUI:** `R0:pass R1:pass R2:fail R3:-- R4:-- R5:-- R6:--` compact format.

### 8.9 Token counter

Formatted token count with magnitude suffix.

Formats: `1.2k` (thousands), `1.2M` (millions). Raw number in tooltip/layer 2.

### 8.10 Freshness bar

Shows Ebbinghaus decay for a knowledge entry.

**Dashboard:** Gradient bar from green (100% fresh) to gray (0% fresh). Tooltip shows exact decay percentage and half-life.
**TUI:** `||||||||..` bar where `|` = fresh, `.` = decayed.

### 8.11 DAG view

Directed acyclic graph renderer for plan task dependencies.

**Dashboard:** SVG or Canvas. Nodes as boxes. Edges as arrows. Color-coded by task status.
**TUI:** ASCII art using `->` for edges, indentation for depth. Already partially implemented in plan_tree widget.

### 8.12 Markdown renderer

Renders markdown content (PRD bodies, research artifacts).

**Dashboard:** `react-markdown` with syntax highlighting for code blocks.
**TUI:** Basic markdown rendering (headers as bold, code blocks with borders, lists with bullets). Already exists in some form for log entries.

### 8.13 Error digest

Compact error summary widget.

**Dashboard:** Red banner with error message, timestamp, and optional retry button.
**TUI:** Already implemented in `widgets/error_digest.rs`.

---

## 9. Data contracts

Complete listing of every API route each page needs. Routes tagged `[existing]` are already implemented in roko-serve. Routes tagged `[new]` need to be built.

### 9.1 Existing routes (no changes needed)

| Route | Method | Used by pages |
|-------|--------|---------------|
| `/api/health` | GET | Conductor, Landing |
| `/api/status` | GET | All pages (regime, agent counts) |
| `/api/metrics` | GET | Conductor |
| `/api/metrics/summary` | GET | Costs, Atelier |
| `/api/metrics/success_rate` | GET | Learning |
| `/api/metrics/engagement` | GET | Learning |
| `/api/metrics/c_factor` | GET | Learning, Agent Overview |
| `/api/metrics/model_efficiency` | GET | Costs, Learning |
| `/api/metrics/gate_rate` | GET | Learning, Plans, Atelier |
| `/api/metrics/experiments` | GET | Learning |
| `/api/metrics/feedback_latency` | GET | Learning |
| `/api/metrics/velocity` | GET | Costs |
| `/api/metrics/coverage` | GET | Atelier |
| `/api/metrics/prometheus` | GET | External monitoring |
| `/api/plans` | GET, POST | Plans, Atelier |
| `/api/plans/{id}` | GET | Plans, Atelier |
| `/api/plans/{id}/execute` | POST | Plans |
| `/api/plans/{id}/status` | GET | Plans, Execution Monitor |
| `/api/plans/generate` | POST | Plans, PRD Browser |
| `/api/prds` | GET | PRD Browser, Research |
| `/api/prds/ideas` | POST | PRD Browser |
| `/api/prds/status` | GET | PRD Browser |
| `/api/prds/{slug}` | GET | PRD Browser |
| `/api/prds/{slug}/draft` | POST | PRD Browser |
| `/api/prds/{slug}/promote` | POST | PRD Browser |
| `/api/prds/{slug}/plan` | POST | PRD Browser |
| `/api/research` | GET | Research |
| `/api/research/topic` | POST | Research |
| `/api/research/enhance-prd/{slug}` | POST | Research |
| `/api/research/enhance-plan/{plan}` | POST | Research |
| `/api/research/enhance-tasks/{plan}` | POST | Research |
| `/api/research/analyze` | POST | Research |
| `/api/managed-agents` | GET | Atelier, Live Agents |
| `/api/agents/register` | POST | Agent identity |
| `/api/agents/{id}` | GET | Agent Overview |
| `/api/agents/{id}/stop` | POST | Agent management |
| `/api/agents/{id}/episodes` | GET | Agent Overview, Ask |
| `/api/agents/{id}/logs` | GET | Agent Overview, Execution |
| `/api/agents/{id}/message` | POST | Ask / Chat |
| `/api/agents/{id}/token` | GET, POST | Keys |
| `/api/agents` | GET | Live Agents, Agent Network (aggregator) |
| `/api/agents/topology` | GET | Agent Network (aggregator) |
| `/api/agents/{id}/stats` | GET | Live Agents, Agent Overview (aggregator) |
| `/api/agents/{id}/skills` | GET | Agent Overview (aggregator) |
| `/api/agents/{id}/heartbeat` | GET | Live Agents (aggregator) |
| `/api/agents/{id}/trace` | GET | Agent Overview (aggregator) |
| `/api/predictions/sessions` | GET | (future) |
| `/api/predictions/sessions/{id}` | GET | (future) |
| `/api/predictions/claims` | GET | (future) |
| `/api/predictions/calibration/{id}` | GET | (future) |
| `/api/knowledge/entries` | GET | Knowledge Graph, Pheromone Field |
| `/api/knowledge/edges` | GET | Knowledge Graph, Pheromone Field |
| `/api/knowledge/search` | GET | Knowledge Graph |
| `/api/knowledge/kinds` | GET | Knowledge Graph |
| `/api/tasks` | GET | (future, aggregator) |
| `/api/tasks/stats` | GET | (future, aggregator) |
| `/api/tasks/{id}` | GET | (future, aggregator) |
| `/api/c-factor/trend` | GET | Learning, Costs |
| `/api/learning/efficiency` | GET | Learning, Costs |
| `/api/learning/cascade-router` | GET | Learning |
| `/api/learning/cascade` | GET | Learning |
| `/api/learning/cost-tiers` | GET | Costs |
| `/api/learning/experiments` | GET | Learning |
| `/api/learning/adaptive-thresholds` | GET | Learning |
| `/api/config` | GET, PUT | Config Editor, Strategy |
| `/api/config/reload` | POST | Config Editor |
| `/api/deployments` | GET, POST | Deploy |
| `/api/deployments/{id}` | GET, DELETE | Deploy |
| `/api/deployments/{id}/logs` | GET | Deploy |
| `/api/deployments/{id}/task` | POST | Deploy |
| `/api/deployments/{id}/callback` | POST | Deploy (internal) |
| `/api/subscriptions` | GET, POST | (future) |
| `/api/subscriptions/{id}` | PUT, DELETE | (future) |
| `/api/subscriptions/{id}/enable` | POST | (future) |
| `/api/subscriptions/{id}/disable` | POST | (future) |
| `/api/templates` | GET, POST | Deploy, Strategy |
| `/api/templates/{name}` | GET, DELETE | Deploy |
| `/api/templates/{name}/deploy` | POST | Deploy |
| `/api/projections/{name}` | GET | (future, StateHub) |
| `/api/projections/{name}/stream` | GET | (future, StateHub SSE) |
| `/api/providers` | GET | Config Editor, Strategy |
| `/api/providers/{id}/health` | GET | Conductor |
| `/api/providers/{id}/test` | POST | Config Editor |
| `/api/models` | GET | Strategy |
| `/api/routing/explain` | GET | Learning |
| `/api/events` | GET (SSE) | Dashboard (real-time events) |
| `/api/diagnosis` | GET | Conductor |
| `/ws` | WS | All real-time features |
| `/api/ws` | WS | Agent relay (aggregator) |

### 9.2 New routes needed

| Route | Method | Crate | Used by | Description |
|-------|--------|-------|---------|-------------|
| `/api/keys` | GET | roko-serve | Keys | List API keys (redacted) |
| `/api/keys` | POST | roko-serve | Keys | Create API key |
| `/api/keys/{id}` | DELETE | roko-serve | Keys | Revoke API key |
| `/api/keys/{id}/rotate` | POST | roko-serve | Keys | Rotate API key |
| `/api/research/{name}` | GET | roko-serve | Research | Read research artifact content |
| `/api/jobs` | GET | roko-serve | Job Board | List jobs from BountyMarket |
| `/api/jobs` | POST | roko-serve | Create Job | Post job to BountyMarket |
| `/api/jobs/{id}` | GET | roko-serve | Job Detail | Get job detail |
| `/api/jobs/{id}/bids` | GET | roko-serve | Job Detail | List bids for a job |
| `/api/jobs/{id}/bid` | POST | roko-serve | Job Detail | Submit bid |
| `/api/jobs/{id}/assign` | POST | roko-serve | Job Detail | Assign worker |
| `/api/jobs/{id}/submit` | POST | roko-serve | Job Detail | Submit work |
| `/api/jobs/{id}/validation` | GET | roko-serve | Job Detail | Get validation state |
| `/api/jobs/{id}/vote` | POST | roko-serve | Job Detail | Validator vote |
| `/api/jobs/types` | GET | roko-serve | Create Job | List job types |
| `/api/jobs/workers` | GET | roko-serve | Job Board | Worker registry |
| `/api/atelier` | GET | roko-serve | Atelier | List ateliers |
| `/api/atelier` | POST | roko-serve | Atelier | Create atelier |
| `/api/atelier/{name}` | GET | roko-serve | Atelier | Get atelier detail |
| `/api/atelier/{name}/stop` | POST | roko-serve | Atelier | Stop atelier |
| `/api/episodes` | GET | roko-serve | Atelier, Agent Overview | List recent episodes (already exists as `/api/status` sub-data, needs standalone) |
| `/api/network/stats` | GET | roko-serve [new — heartbeat.rs] | Agent Network, Landing, Swarm | Returns `{ agents_online, agents_by_domain, total_tasks_completed, avg_cost_per_task, isfr_current }`. Source: aggregated from heartbeat ring buffer. |

### 9.3 WebSocket events (existing)

All emitted by roko-serve's event bus:

| Event type | Fields | Consumers |
|------------|--------|-----------|
| `RunStarted` | run_id, agent_id | Ask, Execution Monitor |
| `RunCompleted` | run_id, success | Ask, Execution Monitor |
| `AgentOutput` | agent_id, run_id, content, done | Ask, Execution Monitor |
| `PlanStarted` | plan_id | Plans, Atelier |
| `PlanCompleted` | plan_id, success | Plans, Atelier |
| `OperationStarted` | op_id, kind | Research, PRD Browser |
| `OperationCompleted` | op_id, kind, success | Research, PRD Browser |
| `Error` | message | All pages (global error banner) |
| `ConfigReloaded` | timestamp | Config Editor |
| `DeploymentCreated` | deployment_id | Deploy |
| `DeploymentFailed` | deployment_id, error | Deploy |

---

## 10. Network intelligence display

### 10.1 ISFR visualization

The ISFR (Intelligence Scaling Factor Ratio) is the primary "is the network getting smarter?" metric. It is a dual-median aggregation across validators with Byzantine fault tolerance.

**Layer 0 display:** Single number (e.g., "1.032") with a trend arrow and sparkline. Color-coded: green > 1.0, amber 0.95-1.0, red < 0.95.

**Layer 1 display:** Breakdown by domain. Trading ISFR: 1.045. Research ISFR: 1.012. Infra ISFR: 0.987.

**Layer 2 display:** Raw validator submissions, median computation, Byzantine exclusions.

### 10.2 C-Factor display

C-Factor is a composite metric measuring agent effectiveness: competence, context efficiency, coherence, creativity, and cost awareness.

**Layer 0:** Single score 0-1 (e.g., "0.82") with traffic-light color.
**Layer 1:** Component breakdown (5 sub-scores).
**Layer 2:** Per-episode C-Factor with all raw inputs.

### 10.3 Knowledge density

How interconnected is the knowledge graph? Measured as edges/node ratio.

**Layer 0:** "Dense" / "Sparse" / "Emerging" label with percentage.
**Layer 1:** Breakdown by kind (playbooks: dense, insights: sparse).
**Layer 2:** Full edge list with weights.

---

## 11. Jobs system integration

### 11.1 Contract addresses

The ERC-8183 jobs system runs on-chain. roko-serve needs to read from these contracts:

- `BountyMarket.sol` -- job lifecycle (post, assign, submit, resolve)
- `WorkerRegistry.sol` -- bonded stake, EMA reputation, tier lookup
- `ConsortiumValidator.sol` -- 3-of-5 committee formation and voting
- `JobTypeRegistry.sol` -- typed job templates with parameter schemas
- `DAEJI.sol` -- ERC-20 token for bounty escrow and worker bonds

### 11.2 Data flow

```
BountyMarket events  -->  mirage-rs (indexer)  -->  roko-serve/jobs.rs  -->  Dashboard
                                                                        -->  Nexus (jobs room)
```

roko-serve's `jobs.rs` module (new) polls mirage-rs for indexed contract events. It does not interact with the contracts directly -- mirage-rs handles on-chain reads and event indexing.

### 11.3 Job lifecycle states

```
Posted -> [Bidding] -> Assigned -> [Working] -> Submitted -> [Validating] -> Resolved
                                                                              |
                                                                        Accept / Reject
```

Each state transition emits a Nexus event on the `jobs` room. Dashboard subscribers update in real-time.

### 11.4 Worker tier mapping

| Tier | Min stake | Rep threshold | Eligible job types |
|------|-----------|---------------|-------------------|
| Probation | 0 DAEJI | < 0.5 | None (observation only) |
| Standard | 1000 DAEJI | >= 0.5 | research, general |
| Trusted | 2500 DAEJI | >= 0.75 | oracle-update, research, general |
| Elite | 5000 DAEJI | >= 0.90 | All including perps-liquidate |

---

## 12. TUI-specific enhancements

### 12.1 New tabs

| Tab | Key | Content | Priority |
|-----|-----|---------|----------|
| F8 Chat | F8 | Interactive agent chat (same as Ask page) | P1 |
| F9 Jobs | F9 | Job board browser (read-only, with bid action) | P2 |

### 12.2 Missing sub-views to implement

These are declared in the TUI code but never rendered:

| Parent tab | Sub-view | What it should show |
|------------|----------|-------------------|
| F6 Config | ProviderHealth | `GET /api/providers` + per-provider health bars |
| F6 Config | ModelComparison | Cost/speed/quality comparison across configured models |
| F7 Inspect | EngramDag | Browse engram graph by hash chain (like `roko replay`) |
| F7 Inspect | EpisodeReplay | Step through episodes chronologically with gate results |
| F7 Inspect | KnowledgeBrowse | Browse knowledge store by kind and freshness |

### 12.2.1 F7 Inspect sub-view organization

F7 Inspect sub-views are accessed via number keys 1-5 when F7 is the active tab:

| Key | Sub-view | Content | Status |
|-----|----------|---------|--------|
| 1 | System Health (default) | Token burn, cost by model, cascade router, alerts | EXISTING |
| 2 | Engram DAG | Interactive engram dependency graph | NEW -- Task 3.3 in IMPL-10 |
| 3 | Episode Replay | Step-through episode turns with gate results | NEW -- Task 3.3 in IMPL-10 |
| 4 | Knowledge Browser | Searchable neuro store by kind and freshness | NEW -- Task 3.3 in IMPL-10 |
| 5 | Conductor | Watcher status, circuit breakers, interventions | NEW -- move from F1 sub-tab |

Notes on this reorganization:
- Costs data (token burn, cost by model, cascade router) remains on the default System Health sub-view (sub-view 1). It is not moved.
- Conductor data moves from being embedded as a sub-section of F1 Dashboard to its own dedicated F7 sub-view (sub-view 5). The F1 Dashboard retains the regime badge and aggregate status dot but does not show watcher detail.
- Sub-views 2, 3, and 4 correspond to the stubbed `EngramDag`, `EpisodeReplay`, and `KnowledgeBrowse` variants in the existing `SubView` enum.

### 12.3 Existing bugs to fix

| Bug | Location | Fix |
|-----|----------|-----|
| `plan_tree` ignores `data` and `view_state` params | `dashboard_view.rs` render_left_panel | Pass data through to plan_tree widget |
| `build_unified_log` runs O(N) per frame | `logs_view.rs` | Cache the unified log, rebuild only on data change (compare JSONL cursor position) |
| `SubView` enum mismatches actual sub-tabs | `dashboard_view.rs` | Align enum variants with `SUB_TAB_LABELS` |
| `vfy` column always shows `*` | `plan_tree` widget | Wire gate result lookup to populate verify status |
| Wave collapse/expand not wired | `plans_view.rs` | Add toggle keybinding and state |
| Topology fetch is one-shot | `agents_view.rs` | Add periodic re-fetch (30s) |
| F5 log entry expand not implemented | `logs_view.rs` | Add Enter key handler to show full entry in modal |
| F5 log search not implemented | `logs_view.rs` | Add `/` keybinding for search, filter entries by query |

### 12.4 Bardo-inspired widgets

These widgets from the Bardo Terminal spec should be ported to the TUI:

| Widget | Description | Priority |
|--------|-------------|----------|
| Spectre sprite | 80-dot particle cloud representing system state | P3 |
| Info density slider | 4-level progressive complexity (casual/standard/advanced/expert) | P2 |
| PAD micro-shifts | Pleasure-Arousal-Dominance color shifts based on regime | P2 |
| Animation timeline | Interpolating variable timeline (like film keyframes) | P3 |
| Braille scatter | Scatter plots using braille characters (2x4 dot grid per cell) | P2 (partially done in `braille.rs`) |

### 12.5 PostFX enhancements

The PostFX pipeline (`postfx.rs`, `postfx_pipeline.rs`) already implements:
- `nerv_viz`: edge glow and grid overlay
- `particles`: drifting particle field
- `bloom`: intensity bloom on bright elements
- `vignette`: corner darkening
- `ambient_orbs`: floating colored orbs
- `dream_atmosphere`: fog and haze effects

Enhancements:

| Effect | Description |
|--------|-------------|
| Regime-responsive intensity | Cruise: minimal effects. Volatile: increased particle speed, warmer colors. Crisis: rapid particles, red glow, screen shake. |
| Focus-following bloom | Bloom tracks the focused element, not global |
| Transition effects | Fade/slide on tab switch, panel resize |

### 12.6 Streaming migration

Current state: TUI polls 7 disk files (episodes.jsonl, engrams.jsonl, efficiency.jsonl, gate-thresholds.json, cascade-router.json, experiments.json, plan files) and re-parses them each tick.

Target state: TUI connects to roko-serve WS `/ws` and receives events in real-time. Disk polling becomes a fallback for when roko-serve is not running.

Migration steps:
1. Add WS client to TUI (already started in `ws_client.rs`)
2. Map WS events to TuiState updates
3. Keep disk polling as fallback (check `GET /api/health` first)
4. Remove per-frame file re-parsing

---

## 13. Dashboard-specific enhancements

### 13.1 Responsive layout

Replace the fixed three-column layout (224px / fluid / 260px) with a responsive design:

| Breakpoint | Layout |
|------------|--------|
| >= 1200px | Three columns (sidebar nav, main content, context panel) |
| 768-1199px | Two columns (collapsible nav, main content) |
| < 768px | Single column (bottom tab bar, main content) |

### 13.2 Design system

Replace the inconsistent spacing and font sizes with a design system:

| Token | Value | Usage |
|-------|-------|-------|
| `--space-1` | 4px | Inline spacing |
| `--space-2` | 8px | Element gaps |
| `--space-3` | 12px | Section padding |
| `--space-4` | 16px | Card padding |
| `--space-5` | 24px | Section margins |
| `--space-6` | 32px | Page padding |
| `--font-xs` | 11px | Labels, badges |
| `--font-sm` | 13px | Secondary text |
| `--font-base` | 15px | Body text |
| `--font-lg` | 18px | Headings |
| `--font-xl` | 24px | Page titles |

All colors from the ROSEDUST palette. Minimum contrast ratio: 4.5:1 for text, 3:1 for interactive elements.

### 13.3 React Router migration

Replace the useState switch/case routing in App.tsx with React Router:

```tsx
<Routes>
  <Route path="/" element={<Landing />} />
  <Route path="/onboarding" element={<Onboarding />} />
  <Route path="/command/chat" element={<ChatPage />} />
  <Route path="/command/research" element={<Research />} />
  <Route path="/observatory/agents" element={<LiveAgents />} />
  <Route path="/observatory/plans" element={<Plans />} />
  <Route path="/observatory/learning" element={<Learning />} />
  <Route path="/observatory/conductor" element={<Conductor />} />
  <Route path="/observatory/costs" element={<Costs />} />
  <Route path="/network/topology" element={<AgentNetwork />} />
  <Route path="/network/pheromones" element={<PheromoneField />} />
  <Route path="/network/knowledge" element={<KnowledgeGraph />} />
  <Route path="/network/swarm" element={<Swarm />} />
  <Route path="/marketplace" element={<JobBoard />} />
  <Route path="/marketplace/create" element={<CreateJob />} />
  <Route path="/marketplace/jobs/:id" element={<JobDetail />} />
  <Route path="/agent-studio/:id/overview" element={<AgentOverview />} />
  <Route path="/agent-studio/:id/strategy" element={<AgentStrategy />} />
  <Route path="/agent-studio/:id/keys" element={<AgentKeys />} />
  <Route path="/agent-studio/:id/deploy" element={<AgentDeploy />} />
  <Route path="/atelier" element={<AtelierDashboard />} />
  <Route path="/atelier/prds" element={<PrdBrowser />} />
  <Route path="/atelier/execution" element={<ExecutionMonitor />} />
  <Route path="/settings/config" element={<ConfigEditor />} />
  <Route path="/settings/theme" element={<Theme />} />
  <Route path="/settings/notifications" element={<Notifications />} />
</Routes>
```

### 13.4 Real-time architecture

Replace the 14+ concurrent polling timers with a unified real-time architecture:

1. **One WS connection** to roko-serve `/ws`. All real-time data flows through this.
2. **One SSE connection** to `/api/events` as fallback (if WS fails).
3. **Polling only for** infrequently changing data (config, templates) with 30s+ intervals.
4. **Request deduplication** -- same endpoint within 1s = single request.
5. **Exponential backoff** on connection failure (1s, 2s, 4s, 8s, max 30s).

### 13.5 Mock data elimination

Every mock data source in the current dashboard must be removed. No hardcoded arrays, no simulated lifecycles, no placeholder strings.

| File | Mock data | Replacement |
|------|-----------|-------------|
| App.tsx | "2,847 PTS", "47 agents", "45,231 block", "6.40% ISFR" | `GET /api/status` |
| AgentOverviewPanel | Mock `agents` array (8 agents) | `GET /api/agents` (aggregator) |
| InsightStoreView | Mock `insights` array appended to live data | Remove mock array entirely |
| ResearchPanel | setTimeout lifecycle simulation | Wire to `POST /api/research/topic` result |
| JobsPanel | `onDrop` never reads files, alert() for artifacts, prompt() for feedback | Wire to new `/api/jobs` routes |
| KeysPanel | No API calls at all (61 lines of static HTML) | Wire to new `/api/keys` routes |
| DeployPanel | Hardcoded binding code "847291" | Dynamic from `POST /api/deployments` |
| NetworkPanel | `refreshPheromones()` not returned from hook | Fix hook to return function |
| All notifications | Static content arrays | WS events |

### 13.6 Navigation

Replace the mode/tab switch-case with a sidebar navigation:

```
+---------+
| nunchi  |
+---------+
| Command |
|  Ask    |
|  Rsrch  |
+---------+
| Observe |
|  Agents |
|  Plans  |
|  Learn  |
|  Cndctr |
|  Costs  |
+---------+
| Network |
|  Agents |
|  Phrmn  |
|  Knwldg |
|  Swarm  |
+---------+
| Market  |
|  Jobs   |
+---------+
| Studio  |
|  Ovrvw  |
|  Strtgy |
|  Keys   |
|  Deploy |
+---------+
| Atelier |
|  Dash   |
|  PRDs   |
|  Exec   |
+---------+
| Settngs |
+---------+
```

Sidebar collapses to icons at `< 1200px`. Disappears at `< 768px` (bottom tab bar replaces it).

---

## 14. Roko stabilization requirements

Before roko can run implementation plans for other PRDs, these issues must be fixed:

### 14.1 Auth middleware upgrade (P0)

roko-serve has ~85 routes with ZERO authentication when `serve.auth.enabled = false` (which is the default). Any process on the same machine can read/write config, start/stop agents, execute plans.

Fix: Enable auth by default. Ship with a generated API key on `roko init`. Add `keys.rs` routes for key management.

### 14.2 In-memory state persistence (P0)

`active_plans`, `discovered_agents`, `operations`, and `deployments` are all in-memory HashMap/RwLock state in AppState. They are lost on restart.

Fix: Persist to `.roko/state/serve-state.json` on every write. Load on startup.

### 14.3 Polling-to-streaming migration (P1)

The TUI polls 7 disk files per tick. This is wasteful and introduces latency.

Fix: Connect TUI to roko-serve WS. The `ws_client.rs` module exists but is not yet wired into the render loop.

### 14.4 Aggregator cache invalidation (P1)

The aggregator caches sidecar responses with fixed TTLs (5-30s). There is no cache invalidation on state change.

Fix: Add WS event-driven cache invalidation. When an agent emits a heartbeat, invalidate its cached stats.

### 14.5 Error handling gaps (P2)

The orchestrator (orchestrate.rs) uses `anyhow::Result` consistently, but some error paths in roko-serve silently swallow errors (particularly in the aggregator fan-out where one sidecar failure should not block the whole response).

Fix: Return partial results with per-agent error indicators.

---

## 15. Demo requirements (Thursday)

### 15.1 Must work

1. **Landing page** renders with real network stats from `GET /api/status`
2. **Atelier dashboard** shows real plan progress, task list, agent count
3. **Live agents** table shows running agents with real data from aggregator
4. **Ask/Chat** sends a message to an agent and shows the streaming response
5. **Plan execution** kicks off from the UI and streams output to Execution Monitor
6. **TUI F1-F7** render real data (already works, verify nothing regressed)

### 15.2 Nice to have

7. Learning page shows real cascade router and experiment data
8. Agent network renders a force-directed graph with real topology
9. PRD browser shows real PRDs and can trigger plan generation
10. Research page launches a real research task

### 15.3 Fallback plan

If real data is unavailable for any section:
- Show "connecting..." skeleton state
- Do NOT show mock data. An empty state is honest. Mock data is a lie.
- Fall back to `GET /api/health` to verify roko-serve is reachable
- If roko-serve is unreachable, show: "Start roko-serve with `roko serve` to see live data."

### 15.4 Demo script

```
1. Open dashboard in browser (localhost:5173)
2. Show landing page -> real agent count, ISFR
3. Click "Get Started" -> navigate to Atelier
4. Show Atelier dashboard -> plan tree, task list
5. Navigate to Observatory/Agents -> show live agent table
6. Navigate to Command/Ask -> send a message, show streaming response
7. Switch to terminal -> show TUI (roko dashboard)
8. F1 -> same data as web dashboard
9. F2 -> show plan DAG
10. F3 -> show agent roster
11. End
```

---

## 16. Cross-references and open questions

### 16.1 Cross-references

| PRD | Relevant sections |
|-----|-------------------|
| PRD-02 (Agent runtime) | Heartbeat format (section 3), CorticalState.regime (section 2.2), agent lifecycle |
| PRD-05 (Knowledge & stigmergy) | Knowledge graph data model, pheromone types, insight store |
| PRD-06 (Domains & arenas) | Domain taxonomy for agent grouping, arena concept for marketplace |
| PRD-07 (ISFR & instruments) | ISFR computation, instrument pricing, validator mechanics |
| PRD-08 (Deployment & UX) | CLI commands, deploy pipeline, `roko login` flow |
| PRD-09 (Extensibility) | MCP server integration, extension model, multi-chain |
| IMPL-08 (Surfaces) | Implementation plan for this PRD |
| IMPL-10 (Dashboard & TUI) | Specific task breakdown for demo deadline |
| IMPL-10 (Demo) | Thursday demo requirements and script |

### 16.2 Open questions

1. **Nexus hosting:** Should Nexus be a separate binary or integrated into roko-serve? Separate is cleaner for federation. Integrated is simpler for self-hosting. Recommendation: start integrated, extract later.

2. **Job board chain dependency:** The jobs system requires mirage-rs to be running and indexing BountyMarket events. For the demo, should we show the job board with "chain not connected" state, or hide it entirely? Recommendation: show it with empty state + explanation.

3. **TUI chat tab:** F8 Chat would require an input mode that captures all keystrokes, conflicting with the current keybinding system (F-keys, letter keys for sub-tabs). How to handle? Recommendation: when chat input is focused, all keys go to the input. `Esc` exits input mode and returns to navigation.

4. **Agent identity without wallet:** Should non-wallet agents get persistent IDs across restarts? Currently, agent IDs are assigned at registration and lost on restart. Recommendation: persist agent registrations in `.roko/state/agents.json`.

5. **Dashboard framework migration:** The current dashboard uses React 19 + Vite 8 + Tailwind v4. Should we consider Next.js for SSR, or keep it a pure SPA? Recommendation: keep SPA. roko-serve is the backend. Adding SSR adds complexity with no clear benefit for a monitoring dashboard.

---

*End of PRD-10.*
