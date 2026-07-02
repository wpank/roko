# Roko architecture redesign (v2)

## The problem

Roko has four infrastructure components that evolved independently and never agreed on boundaries:

1. **Mirage** -- a devnet chain with a relay WebSocket. Always on, shared across users.
2. **roko-serve** -- an HTTP control plane with ~85 routes. Requires a workspace directory. Optional for users who only want agents.
3. **roko-agent-server** -- a per-agent HTTP sidecar (13 routes). One process per agent. Breaks behind NAT.
4. **Dashboard / TUI** -- consumes REST endpoints from all three. Falls over when any backend is unreachable.

This creates several concrete failures:

- **Per-agent sidecars don't traverse NAT.** An agent on a Fly Machine can't expose an HTTP server that the control plane can reach without proxy configuration. The sidecar model assumes a flat network.
- **Dashboard requires roko-serve.** If the control plane is down, the dashboard shows "Backend offline" even though agents may still be running and the relay still has presence data.
- **Polling everywhere.** The dashboard polls multiple endpoints on 1-5 second intervals. This wastes bandwidth, creates visual jitter, and scales poorly with agent count.
- **API keys scattered.** Each agent holds its own LLM API keys via environment variables. No central audit, no rotation, no cost attribution.
- **No agent lifecycle.** Agents are either ephemeral CLI processes or stateless HTTP workers. No heartbeat, no mode (persistent vs reactive), no graceful shutdown protocol.
- **Three discovery sources, zero merge.** Relay presence, ERC-8004 on-chain registry, and manually-added deployment URLs each live in separate UIs. No unified agent list.

This document specifies the architecture that resolves all six.

---

## Architecture overview

```
                         ┌─────────────────────────┐
                         │   Mirage chain + Relay   │  Always on. Shared.
                         │   (mirage-devnet.fly.dev)│
                         │                          │
                         │  Chain: blocks, events,  │
                         │         ERC-8004 registry │
                         │  Relay: agent presence,  │
                         │         WS event routing  │
                         └────────┬─────────────────┘
                                  │ WebSocket
             ┌────────────────────┼─────────────────────────┐
             │                    │                          │
             ▼                    ▼                          ▼
  ┌──────────────────┐  ┌─────────────────────┐   ┌──────────────────┐
  │    Dashboard     │  │    roko process      │   │  Remote agent    │
  │   (web / TUI)    │  │   (optional)         │   │  (Fly / Railway) │
  │                  │  │                      │   │                  │
  │ Connects to:     │  │  ┌───────────────┐   │   │  Connects        │
  │ - Relay (always) │  │  │ Control plane │   │   │  OUTBOUND to     │
  │ - roko (if avail)│  │  │ (roko-serve)  │   │   │  relay via WS    │
  │ - Agent feeds    │  │  ├───────────────┤   │   │                  │
  │                  │  │  │ Agent runtime │   │   │  Gets inference  │
  │ Subscribes to WS │  │  │ (tokio tasks) │   │   │  via parent      │
  │ per page. No     │  │  ├───────────────┤   │   │  gateway proxy   │
  │ polling.         │  │  │ Inference     │   │   │                  │
  └──────────────────┘  │  │ Gateway       │   │   └──────────────────┘
                        │  └───────────────┘   │
                        │                      │
                        │  In-process agents:   │
                        │  ┌─────┐ ┌─────┐     │
                        │  │ A1  │ │ A2  │ ... │
                        │  └─────┘ └─────┘     │
                        └──────────────────────┘
```

Three deployment tiers:

| Tier | What runs | Who needs it |
|------|-----------|--------------|
| **Backbone** | Mirage chain + relay | Everyone. Always on. Shared infrastructure. |
| **Workspace** | roko process (control plane + agent runtime + inference gateway) | Users who want orchestration, plans, PRDs, learning. |
| **Remote agents** | Standalone processes on Fly/Railway | Users who need isolation or scale. |

The backbone is the only hard dependency. Everything else is additive.

---

## Data flow: subscription-only

Every piece of data flows through WebSocket subscriptions. No polling.

### Event sources

```
Source              Transport           What it carries
──────              ─────────           ───────────────
Relay               WS /relay/ws        Agent presence, message lifecycle,
                                        relay health
roko-serve          WS /ws              Plan progress, gate results, episodes,
                                        learning metrics, job updates
Agent (direct)      WS (per-agent)      Heartbeats, streaming LLM output,
                                        decision traces
Agent (via relay)   WS /relay/ws        Same as direct, tunneled through relay
Chain               WS (RPC sub)        Blocks, contract events, ERC-8004
                                        registry updates
Agent chain feeds   WS (per-feed)       Raw RPC data, derived indicators,
                                        signals, analysis
```

### Subscription lifecycle

The dashboard subscribes when a page mounts and unsubscribes when it unmounts.

```typescript
// React hook pattern
function useAgentFeed(agentId: string) {
  const [state, setState] = useState<AgentState | null>(null);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);

    ws.onopen = () => {
      // Subscribe to this agent's room
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
    };

    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      setState(prev => applyEvent(prev, event));
    };

    return () => {
      ws.send(JSON.stringify({
        type: "unsubscribe",
        rooms: [`agent:${agentId}`, `agent:${agentId}:heartbeat`]
      }));
      ws.close();
    };
  }, [agentId]);

  return state;
}
```

### WebSocket message envelope

Every message through the relay uses the same envelope:

```json
{
  "seq": 4821,
  "ts": 1713974400123,
  "room": "agent:coder-1:heartbeat",
  "type": "heartbeat",
  "payload": { ... }
}
```

| Field | Type | Purpose |
|-------|------|---------|
| `seq` | `u64` | Monotonic sequence number per connection. Enables reconnection replay. |
| `ts` | `u64` | Unix milliseconds. Server clock. |
| `room` | `string` | Scoping. Clients subscribe to rooms, receive only matching messages. |
| `type` | `string` | Event discriminant. One of the types listed below. |
| `payload` | `object` | Type-specific data. |

### Room naming convention

```
agent:{id}                  Agent lifecycle events (spawn, stop, error)
agent:{id}:heartbeat        Heartbeat ticks (T0/T1/T2, cortical state)
agent:{id}:output           Streaming LLM output chunks
agent:{id}:trace            Decision traces per tick
agent:{id}:feed:{feed_id}   Chain data feeds exposed by the agent
plan:{id}                   Plan progress, task transitions, gate results
cluster:{id}                Cluster pipeline events
system                      Server health, provider status, cost updates
learning                    Experiment results, router updates, thresholds
```

### Event types

```
Type                    Room pattern            Payload
────                    ────────────            ───────
presence_join           system                  { agent_id, mode, profile }
presence_leave          system                  { agent_id, reason }
heartbeat               agent:{id}:heartbeat    { tick, tier, pe, cortical_state }
output_chunk            agent:{id}:output       { content, done, usage }
trace                   agent:{id}:trace        { tick, steps[], gate_result }
task_started            plan:{id}               { task_id, phase }
task_completed          plan:{id}               { task_id, outcome }
gate_result             plan:{id}               { task_id, gate, rung, passed }
phase_transition        plan:{id}               { from, to }
feed_data               agent:{id}:feed:{fid}   { feed_id, data }
feed_registered         system                  { agent_id, feed_id, schema }
cost_update             system                  { agent_id, delta, total }
provider_status         system                  { provider, healthy, latency_ms }
experiment_result       learning                { experiment_id, winner, p_value }
router_update           learning                { model, weight, reason }
```

### Backpressure and coalescing

High-frequency events (heartbeats at 100ms, chain blocks at 2s) need throttling for dashboard consumption.

```
Strategy                Applies to              Behavior
────────                ──────────              ────────
Coalesce                heartbeat               Relay buffers heartbeats per agent,
                                                sends latest every 500ms to
                                                dashboard subscribers
Drop-oldest             output_chunk            Ring buffer per agent (1024 chunks).
                                                Slow consumers miss old chunks,
                                                catch up from latest.
Lossless                gate_result,            Queue with backpressure. If client
                        task_completed          can't keep up, relay applies
                                                TCP-level flow control.
Sample                  feed_data               Agent-configurable sample rate.
                                                Default: every Nth update where
                                                N = ceil(source_rate / 2Hz).
```

### Reconnection

Clients track the last received `seq`. On reconnect:

```json
{ "type": "resume", "last_seq": 4821 }
```

The relay replays missed events from its ring buffer (default: 64K entries, ~10 minutes at moderate throughput). If the gap exceeds the buffer, the relay sends a `snapshot` event with current state followed by live events.

---

## Agent runtime

### The AgentRuntime struct

Every agent -- in-process or remote -- runs the same core loop.

```rust
pub struct AgentRuntime {
    /// Unique agent identifier.
    pub id: AgentId,
    /// Human-readable name.
    pub name: String,
    /// Domain profile (user-defined string, e.g. "coding", "chain", "defi-trader").
    pub profile: DomainProfile,  // newtype over String
    /// Lifecycle mode.
    pub mode: AgentMode,
    /// The 9-step heartbeat pipeline.
    pipeline: TickPipeline,
    /// Cortical state: working memory, goals, beliefs, attention.
    cortical: CorticalState,
    /// Extension chain (ordered list of hooks).
    extensions: Vec<Box<dyn Extension>>,
    /// Inbound message queue.
    inbox: mpsc::Receiver<AgentMessage>,
    /// Handle to the centralized inference gateway.
    inference: InferenceHandle,
    /// Handle to the relay for presence and event publishing.
    relay: RelayHandle,
    /// Adaptive clock controlling tick frequency.
    clock: AdaptiveClock,
    /// Cancellation token for graceful shutdown.
    cancel: CancellationToken,
}
```

### The run() loop

```rust
impl AgentRuntime {
    pub async fn run(mut self) -> AgentResult {
        self.relay.announce_presence(&self.id, &self.profile).await;

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => break,
                _ = self.clock.tick() => {
                    let result = self.pipeline.execute_tick(
                        &mut self.cortical,
                        &self.extensions,
                        &self.inference,
                    ).await;

                    self.relay.publish_heartbeat(&self.id, &result).await;

                    if result.should_stop() {
                        break;
                    }
                }
                msg = self.inbox.recv() => {
                    if let Some(msg) = msg {
                        self.handle_message(msg).await;
                    }
                }
            }
        }

        self.relay.announce_leave(&self.id).await;
        self.cortical.into_result()
    }
}
```

### The 9-step pipeline

Each tick executes these steps in order. Extensions can intercept at each step.

```
Step        Name        What happens
────        ────        ────────────
1           Observe     Read inbox, check triggers, scan environment.
2           Retrieve    Query neuro store, load relevant context.
3           Analyze     Score observations, compute prediction error.
4           Gate        T0/T1/T2 decision. High PE → T2 (full reasoning).
                        Low PE → T0 (fast reflex). Budget exceeded → sleepwalk.
5           Simulate    If T1+: generate candidate actions, evaluate outcomes.
6           Validate    Safety checks, capability verification, budget guard.
7           Execute     Dispatch action (LLM call, tool use, message send).
8           Verify      Check execution result against predictions.
9           Reflect     Update cortical state, log episode, adjust clock.
```

### Three modes

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentMode {
    /// Runs until task completes, then stops.
    Ephemeral,
    /// Runs continuously until manually stopped.
    Persistent,
    /// Sleeps until a trigger fires, wakes, works, sleeps again.
    Reactive,
}
```

**Ephemeral**: the default for task-oriented work. The agent receives a task, executes it through the pipeline, and shuts down when done. Use cases: coding tasks, one-off research, PR review.

**Persistent**: the agent runs its tick loop indefinitely. It processes messages from its inbox, monitors its environment, and maintains long-running state. Use cases: chain monitoring, continuous integration watchers, team coordinators.

**Reactive**: the agent registers triggers (webhooks, cron schedules, chain events, messages) and sleeps. When a trigger fires, the runtime wakes the agent, it processes the event through the full pipeline, then sleeps again. Zero compute cost while sleeping.

```toml
# roko.toml -- reactive agent example
[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
  { type = "webhook", path = "/hooks/github-pr" },
  { type = "schedule", cron = "0 9 * * MON" },   # Monday morning sweep
]
```

### Three timescales

The adaptive clock operates at three frequencies:

| Timescale | Name | Frequency | Purpose |
|-----------|------|-----------|---------|
| Gamma | Fast perception | 100ms - 1s | Reflex responses, environment scanning, heartbeat |
| Theta | Reflective planning | 5s - 30s | Reasoning, strategy adjustment, context retrieval |
| Delta | Deep consolidation | 1m - 10m | Memory consolidation, model updates, knowledge distillation |

The clock adapts based on prediction error and activity. High PE → faster ticks. Low PE → slower ticks. No activity → delta mode (conserve resources).

### T0/T1/T2 gating

Each tick decides how much reasoning to apply:

```
Input: prediction_error (PE), budget_remaining, cortical_urgency

T0 (reflex):     PE < 0.15 AND no urgent messages
                  → Skip steps 5-6, execute cached/habitual action
                  → Cost: ~0 tokens (no LLM call)

T1 (reflective): PE 0.15-0.40 OR moderate urgency
                  → Run steps 5-6 with lightweight model (Haiku)
                  → Cost: ~500 tokens

T2 (deliberate): PE > 0.40 OR high urgency OR novel situation
                  → Full pipeline with capable model (Sonnet/Opus)
                  → Cost: ~2000-8000 tokens

Sleepwalk:        Budget exhausted OR externally throttled
                  → Steps 1, 9 only (observe + reflect)
                  → Cost: 0 tokens
```

### Extension chain

Agents are specialized by their extension chain, not code forks. Extensions implement hooks across eight layers:

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Which layer this extension operates in.
    fn layer(&self) -> ExtensionLayer;

    // --- Foundation layer ---
    async fn on_init(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }

    // --- Perception layer ---
    async fn on_observe(&self, _obs: &mut Observations) -> Result<()> { Ok(()) }
    async fn filter_input(&self, _input: &mut AgentMessage) -> Result<FilterDecision> {
        Ok(FilterDecision::Pass)
    }

    // --- Memory layer ---
    async fn on_retrieve(&self, _query: &str, _results: &mut Vec<MemoryItem>) -> Result<()> {
        Ok(())
    }
    async fn on_store(&self, _item: &MemoryItem) -> Result<()> { Ok(()) }

    // --- Cognition layer ---
    async fn pre_inference(&self, _req: &mut InferenceRequest) -> Result<()> { Ok(()) }
    async fn post_inference(&self, _resp: &mut InferenceResponse) -> Result<()> { Ok(()) }
    async fn on_gate(&self, _decision: &mut GateDecision) -> Result<()> { Ok(()) }

    // --- Action layer ---
    async fn pre_action(&self, _action: &mut Action) -> Result<ActionDecision> {
        Ok(ActionDecision::Proceed)
    }
    async fn post_action(&self, _action: &Action, _result: &ActionResult) -> Result<()> {
        Ok(())
    }
    async fn on_tool_call(&self, _call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }

    // --- Social layer ---
    async fn on_message_send(&self, _msg: &mut AgentMessage) -> Result<()> { Ok(()) }
    async fn on_message_receive(&self, _msg: &AgentMessage) -> Result<()> { Ok(()) }

    // --- Meta layer ---
    async fn on_reflect(&self, _state: &CorticalState) -> Result<Vec<Adjustment>> {
        Ok(vec![])
    }
    async fn on_cost_update(&self, _usage: &Usage) -> Result<()> { Ok(()) }

    // --- Recovery layer ---
    async fn on_error(&self, _error: &AgentError) -> Result<RecoveryAction> {
        Ok(RecoveryAction::Propagate)
    }
    async fn on_budget_exceeded(&self, _usage: &Usage) -> Result<BudgetAction> {
        Ok(BudgetAction::Sleepwalk)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExtensionLayer {
    Foundation,
    Perception,
    Memory,
    Cognition,
    Action,
    Social,
    Meta,
    Recovery,
}
```

### Domain profiles

Domains are not hardcoded. A profile is just a string label with a default set of extensions and tools. Roko ships a handful of built-in profiles, but users create their own by declaring them in config or code. Any profile name is valid.

```rust
/// A domain profile is a user-defined string, not an enum.
/// Built-in profiles provide convenience defaults; custom profiles
/// are first-class and work identically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile(pub String);
```

Built-in profiles ship default extension sets as a convenience:

| Built-in profile | Default extensions | Default tools |
|---------|-----------|---------------|
| `coding` | git, compiler, test-runner, lsp | bash, file_edit, git, grep |
| `research` | web-search, citation, summarizer | web_search, pdf_read, cite |
| `chain` | chain-reader, tx-builder, feed-publisher | eth_call, send_tx, subscribe_events |

But there is nothing special about these. A user can define any profile:

```toml
# Custom profile — no built-in knowledge needed
[[agents]]
name = "security-auditor"
profile = "security"        # user-defined, not in any enum
mode = "reactive"
extensions = ["code-scanner", "vuln-db", "report-writer"]
tools = ["grep", "ast_query", "file_read", "web_search"]
triggers = [{ type = "webhook", path = "/hooks/github-pr" }]

[[agents]]
name = "music-composer"
profile = "creative"        # another user-defined profile
mode = "persistent"
extensions = ["midi-gen", "audio-analysis", "feed-publisher"]
feeds = [
  { id = "ambient-soundscape", kind = "derived", schema = "audio_stream_v1", rate_hz = 1.0, access = "public" },
]
```

Profiles with no built-in defaults simply start with an empty extension chain -- the user specifies everything explicitly via `extensions` and `tools`. The extension system is plug-and-play: drop extension code into a known path, reference it by name in config.

Users can also publish profiles as shareable configs:

```toml
# ~/.roko/profiles/defi-trader.toml
[profile]
name = "defi-trader"
description = "DeFi trading agent with risk management and P&L tracking"
extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker", "feed-publisher"]
tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]
default_mode = "persistent"
default_budget = { daily_limit_usd = 50.0 }
```

Then reference it:

```toml
[[agents]]
name = "my-trader"
profile = "defi-trader"   # loads from ~/.roko/profiles/defi-trader.toml
mode = "persistent"
```

Extensions themselves are also user-authored. The `Extension` trait (22 hooks, 8 layers) is the composition boundary -- implement the hooks you need, ignore the rest, and your extension plugs into any agent regardless of profile.

---

## Agent connectivity

Agents communicate across users and across machines. The relay is the rendezvous point -- any agent connected to the relay can discover and message any other agent, regardless of who owns them. There is no concept of "my agents" vs "their agents" at the protocol level. Ownership and access control are handled by auth (Privy JWT, wallet signatures, API keys), not by network isolation.

```
User A's roko process          Relay            User B's Fly Machine
┌──────────────┐                                ┌──────────────┐
│ agent-alpha  │──── WS ────►┌────────┐◄── WS ──│ agent-beta   │
│              │             │ Relay  │          │              │
│ Can message  │◄── relay ───│        │── relay─►│ Can message  │
│ agent-beta   │  forwarding │        │forwarding│ agent-alpha  │
└──────────────┘             │        │          └──────────────┘
                             │        │
User C's dashboard           │        │          User D's agent (local)
┌──────────────┐             │        │          ┌──────────────┐
│ Dashboard    │──── WS ────►│        │◄── WS ──│ agent-gamma  │
│ sees all 3   │             └────────┘          │ behind NAT   │
│ agents       │                                 └──────────────┘
└──────────────┘
```

Cross-user agent communication patterns:

- **Direct messaging**: Agent A sends a message to Agent B via relay. B receives it in its inbox, processes it during the next tick, and can respond.
- **Feed subscription**: Agent A subscribes to Agent B's feed (free or paid). Data flows B → relay → A continuously.
- **Pheromone signaling**: Agents deposit pheromones on-chain. Any agent can read them -- stigmergic coordination without explicit messaging.
- **Cluster collaboration**: Agents from different users can join the same cluster if authorized. The cluster pipeline orchestrates them together.
- **Knowledge sharing**: Agents publish knowledge to the InsightStore (on-chain). Any agent can query it regardless of owner.

Auth controls what an agent can do, not who it can talk to:

| Action | Auth required |
|--------|--------------|
| Discover agents on relay | None (public) |
| Read agent card / capabilities | None (public) |
| Send message to agent | Privy JWT or API key |
| Subscribe to free feed | None |
| Subscribe to paid feed | MPP session or x402 payment |
| Join a cluster | Cluster owner's invitation token |
| Read on-chain knowledge | None (public chain data) |
| Publish knowledge on-chain | Agent wallet signature |

### In-process agents

The default. Agents run as tokio tasks inside the roko process. Communication happens through channels.

```
┌──────────────────────────────────────────────────────────────┐
│                        roko process                          │
│                                                              │
│  ┌──────────────┐     mpsc          ┌──────────────────┐    │
│  │ Control      │ ◄──────────────── │ AgentRuntime     │    │
│  │ Plane        │ ────────────────► │ "coder-1"        │    │
│  │              │     mpsc          │                   │    │
│  │              │                   │ Extensions:       │    │
│  │ Routes msgs  │     mpsc          │  - GitExt         │    │
│  │ to agents    │ ◄──────────────── │  - CompilerExt    │    │
│  │ via channel  │ ────────────────► │  - TestRunnerExt  │    │
│  │ map          │     mpsc          │                   │    │
│  │              │                   └──────────────────┘    │
│  │  agent_id →  │                                           │
│  │  Sender      │     mpsc          ┌──────────────────┐    │
│  │              │ ◄──────────────── │ AgentRuntime     │    │
│  └──────────────┘ ────────────────► │ "research"       │    │
│                       mpsc          └──────────────────┘    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ Inference Gateway                                     │    │
│  │ (shared by all in-process agents via InferenceHandle) │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

Benefits: zero serialization overhead, shared inference gateway, shared memory structures, no network latency.

### Remote agents

For isolation or NAT traversal, agents connect OUTBOUND to the relay. No inbound server required.

```
┌───────────────┐         ┌─────────────┐        ┌───────────────┐
│ Remote agent  │ ──WS──► │   Relay     │ ◄──WS──│ Control plane │
│ (Fly Machine) │         │             │        │ (roko-serve)  │
│               │         │ Routes msgs │        │               │
│ Connects out. │         │ by agent_id │        │ Routes msgs   │
│ No inbound    │         │             │        │ to relay for  │
│ ports needed. │         │             │        │ remote agents │
└───────────────┘         └─────────────┘        └───────────────┘
```

The relay acts as a message router. Both the agent and the control plane maintain persistent WebSocket connections to the relay. Messages are routed by agent ID.

Remote agent startup:

```bash
# On the Fly Machine / Railway container
roko agent run \
  --name "isolated-coder" \
  --relay wss://relay.nunchi.dev \
  --inference-proxy https://my-roko.up.railway.app/api/inference \
  --auth-token $AGENT_TOKEN
```

The agent:
1. Connects to the relay WebSocket
2. Announces presence with its agent ID and capabilities
3. Enters the standard `run()` loop
4. Routes inference requests to the parent's gateway via HTTPS proxy
5. Publishes heartbeats and events through the relay

### Direct-reachable agents

Some remote agents have public URLs (Railway services, dedicated VMs). These can receive messages directly via HTTP in addition to the relay path.

```toml
# User's local deployment list (stored in dashboard localStorage or roko.toml)
[[remote_agents]]
name = "staging-monitor"
url = "https://staging-monitor.fly.dev"
auth_token_ref = "secrets.staging_monitor_token"
```

The control plane prefers direct HTTP for request-response patterns (lower latency) and uses the relay for event streaming and presence.

### Agent discovery: three sources merged

```
┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│ Relay presence   │  │ ERC-8004 on-chain│  │ User's deployment  │
│                  │  │ registry         │  │ list               │
│ Who's online     │  │                  │  │                    │
│ right now.       │  │ Wallet address,  │  │ Railway/Fly URLs,  │
│                  │  │ reputation,      │  │ manually added     │
│ Source of truth  │  │ stake, caps,     │  │ endpoints.         │
│ for liveness.    │  │ feed adverts.    │  │                    │
│                  │  │                  │  │ Per-user. Stored   │
│ Always available.│  │ Source of truth   │  │ in localStorage.   │
│                  │  │ for identity +   │  │                    │
│                  │  │ feed discovery.  │  │ Always available.  │
│                  │  │                  │  │                    │
│                  │  │ Always available. │  │                    │
└────────┬────────┘  └────────┬─────────┘  └────────┬───────────┘
         │                    │                      │
         └────────────────────┼──────────────────────┘
                              ▼
                   ┌─────────────────────┐
                   │ Merged agent list   │
                   │                     │
                   │ Each agent has:     │
                   │ - id, name          │
                   │ - online (relay)    │
                   │ - reputation (chain)│
                   │ - endpoints (deploy)│
                   │ - capabilities      │
                   │ - mode, profile     │
                   │ - feeds (chain+relay│
                   └─────────────────────┘
```

The dashboard merges all three sources client-side. The relay provides liveness. The chain provides identity and reputation. The deployment list provides connectivity.

```typescript
interface MergedAgent {
  id: string;
  name: string;

  // From relay
  online: boolean;
  lastSeen: number;
  mode: "ephemeral" | "persistent" | "reactive";
  profile: string;

  // From chain (ERC-8004)
  wallet?: string;
  reputation?: number;
  stake?: bigint;
  tier?: "gray" | "copper" | "silver" | "gold" | "amber";
  capabilities?: string[];
  cardUri?: string;
  feeds?: FeedAdvertisement[];  // feeds registered in passport

  // From deployment list
  directUrl?: string;
  deployPlatform?: "fly" | "railway" | "manual";
}

interface FeedAdvertisement {
  feedId: string;
  schema: string;
  rateHz: number;
  access: "public" | { paid: { pricePerHour: number } };
  description: string;
}
```

### Message routing

The control plane routes messages based on agent location:

```rust
impl ControlPlane {
    async fn send_to_agent(&self, agent_id: &AgentId, msg: AgentMessage) -> Result<()> {
        // 1. Check in-process agents first (fastest path)
        if let Some(sender) = self.local_agents.get(agent_id) {
            return sender.send(msg).await.map_err(Into::into);
        }

        // 2. Check direct-reachable agents (HTTP)
        if let Some(url) = self.deployment_urls.get(agent_id) {
            return self.http_client
                .post(format!("{url}/api/message"))
                .json(&msg)
                .send()
                .await
                .map_err(Into::into);
        }

        // 3. Fall back to relay (works for NAT-traversal)
        self.relay.send(agent_id, msg).await
    }
}
```

---

## Agent feeds

Any agent can produce and consume real-time data feeds. Blockchain agents subscribe to RPC WebSocket feeds; research agents ingest papers and web data; coding agents emit build status and metrics. The feed system is domain-agnostic -- the examples below use blockchain because that is the most data-intensive case, but the same extension pattern works for any data source.

### Blockchain: raw feed subscription

### Raw feed subscription

```rust
// Inside a ChainReaderExt extension
impl ChainReaderExt {
    async fn on_init(&mut self, ctx: &mut AgentContext) -> Result<()> {
        // Subscribe to Ethereum mainnet new blocks
        let eth_blocks = ctx.chain_subscribe(
            "ethereum",
            "newHeads",
            json!({}),
        ).await?;

        // Subscribe to Uniswap V3 swap events on Base
        let base_swaps = ctx.chain_subscribe(
            "base",
            "logs",
            json!({
                "address": "0x...",  // Uniswap V3 factory
                "topics": ["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"]
            }),
        ).await?;

        self.feeds.push(eth_blocks);
        self.feeds.push(base_swaps);
        Ok(())
    }
}
```

### Exposing feeds externally

Any agent -- regardless of domain -- can register feeds with the relay. The relay handles discovery, subscription routing, and payment gating. The agent just publishes data; the relay does the rest.

```rust
// Agent publishes a raw feed
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-mainnet-blocks",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Raw,
    schema: FeedSchema::EthBlock,
    rate_hz: 0.08,  // ~1 block per 12s
    access: FeedAccess::Public,
})?;

// Agent publishes a derived feed (computed from raw data)
ctx.relay.register_feed(FeedRegistration {
    feed_id: "eth-gas-trend",
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("gas_trend_v1"),
    rate_hz: 0.5,
    access: FeedAccess::Paid { price_per_hour: 100 },  // 100 units/hr
})?;
```

### Feed discovery

The relay maintains a feed registry. Dashboards and agents discover feeds dynamically.

```
GET /relay/feeds
→ [
    {
      "feed_id": "eth-mainnet-blocks",
      "agent_id": "chain-watcher-1",
      "kind": "raw",
      "schema": "eth_block",
      "rate_hz": 0.08,
      "access": "public",
      "subscribers": 3
    },
    {
      "feed_id": "eth-gas-trend",
      "agent_id": "chain-watcher-1",
      "kind": "derived",
      "schema": "gas_trend_v1",
      "rate_hz": 0.5,
      "access": { "paid": { "price_per_hour": 100 } },
      "subscribers": 1
    }
  ]
```

### Dashboard subscribes to agent chain feeds

```typescript
function useChainFeed(agentId: string, feedId: string) {
  const [data, setData] = useState<FeedData[]>([]);

  useEffect(() => {
    const ws = new WebSocket(`${relayUrl}/relay/ws`);
    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "subscribe",
        rooms: [`agent:${agentId}:feed:${feedId}`]
      }));
    };
    ws.onmessage = (e) => {
      const event = JSON.parse(e.data);
      if (event.type === "feed_data") {
        setData(prev => [...prev.slice(-999), event.payload]);
      }
    };
    return () => ws.close();
  }, [agentId, feedId]);

  return data;
}
```

### Agent-to-agent feed marketplace

Agents can subscribe to each other's feeds, creating a data marketplace:

```rust
// Agent B subscribes to Agent A's derived feed
let subscription = ctx.relay.subscribe_feed(SubscribeFeedRequest {
    feed_id: "eth-gas-trend",
    source_agent_id: "chain-watcher-1",
})?;

// For paid feeds, payment is handled automatically via the inference gateway's
// cost tracking. The subscribing agent's budget is debited per hour.
```

### Dynamic endpoint registration

Agents can register new endpoints at runtime. When an agent discovers a new data source or creates a derived feed, it announces it to the relay.

```rust
// Agent discovers a new DEX and creates a feed for it
ctx.relay.register_feed(FeedRegistration {
    feed_id: format!("dex-{}-swaps", dex_address),
    agent_id: ctx.agent_id.clone(),
    kind: FeedKind::Derived,
    schema: FeedSchema::Custom("dex_swap_v1"),
    rate_hz: 2.0,
    access: FeedAccess::Public,
})?;

// The dashboard discovers this feed dynamically
// because it subscribes to the "system" room and receives
// feed_registered events
```

---

## Paid feeds and agent services

The previous section covers how agents subscribe to chain data and expose feeds via the relay. This section covers payment: how a feed producer sets a price, how subscribers pay, how sessions work, and how the dashboard surfaces it all.

### Payment protocols

Two protocols, both implemented in bardo (`crates/mpp/`).

**x402 (per-request, stateless)**

The simplest payment flow. No session, no state. Each request carries its own authorization.

```
Client                                  Server (relay / agent)
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │ ──────────────────────────────────────> │
  │                                         │
  │  HTTP 402                               │
  │  X-Payment-Required:                    │
  │    amount=50, recipient=0xABC...,       │
  │    nonce=1, expiry=1714000000           │
  │ <────────────────────────────────────── │
  │                                         │
  │  Client signs ERC-3009 authorization    │
  │  (gasless USDC approval, no on-chain tx)│
  │                                         │
  │  GET /relay/feeds/eth-gas-trend/data    │
  │  X-Payment: <signed authorization>      │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Server verifies signature (ecrecover,  │
  │  no RPC needed), serves content         │
  │                                         │
  │  200 OK + feed data                     │
  │ <────────────────────────────────────── │
```

Settlement happens in batches: every 10 minutes or after 100+ accumulated authorizations, whichever comes first. The server submits a single on-chain transaction that settles all pending authorizations. This amortizes gas costs across many payments.

**MPP (session-based, streaming)**

For continuous feeds. One signature funds an entire session. No re-signing per message.

```
Client                                  Server (relay / agent)
  │                                         │
  │  POST /mpp/sessions                     │
  │  { amount: 500, authorization: <sig> }  │
  │ ──────────────────────────────────────> │
  │                                         │
  │  201 Created                            │
  │  { session_id: "abc-123",               │
  │    funded: 500, status: "active" }      │
  │ <────────────────────────────────────── │
  │                                         │
  │  WS subscribe with session_id           │
  │  { rooms: ["feed:eth-gas-trend"],       │
  │    payment: { session_id: "abc-123" } } │
  │ ──────────────────────────────────────> │
  │                                         │
  │  Per-message draw from session          │
  │  (no client interaction needed)         │
  │                                         │
  │  feed_data: { ema_12: 42.5, ... }       │
  │  payment_draw: { amount: 1,             │
  │    balance_remaining: 499 }             │
  │ <────────────────────────────────────── │
```

Session lifecycle:

```
Active ──> Exhausted ──> Expired ──> Settled
  │            │                       │
  │  (top-up)  │                       │
  └────────────┘                       │
                                       └── Refund unspent balance
```

- **Active**: draws succeed, messages flow.
- **Exhausted**: balance hits zero. Server sends exhaustion notice and pauses delivery. Client can top-up to resume.
- **Expired**: TTL reached (default 24h). No more draws. Transitions to Settled.
- **Settled**: unspent balance refunded. Session closed. Settlement submitted on-chain.

**When to use which**

| Scenario | Protocol | Why |
|----------|----------|-----|
| Try a feed for 5 minutes | x402 | No session overhead, pay per message |
| Subscribe to a price feed for 24h | MPP | One signature, draws per tick |
| Query an agent's analysis on-demand | x402 | Stateless, pay per query |
| Multi-agent pipeline consuming feeds | MPP | Pre-funded sessions per pipeline stage |

**Reputation-based pricing**

Higher ERC-8004 reputation tier = lower markup. Applied on top of the feed producer's base price.

| Tier | Markup |
|------|--------|
| None | +20% |
| Basic | +18% |
| Verified | +15% |
| Trusted | +12% |
| Sovereign | +8% |

A feed priced at $0.10/hr costs a `None`-tier subscriber $0.12/hr and a `Sovereign`-tier subscriber $0.108/hr. The spread goes to the relay as an infrastructure fee.

### Setting up a paid feed

A concrete walkthrough: building a "gas-oracle" agent that produces a paid ETH gas trend feed.

**Step 1: Declare the feed in the agent manifest**

```toml
# roko.toml -- agent manifest
[agent]
name = "gas-oracle"
profile = "chain"
mode = "persistent"

[agent.feeds]
[agent.feeds.eth-gas-trend]
kind = "derived"
description = "12-block EMA gas price with percentile bands and MEV spike detection"
schema = "gas_trend_v1"
rate_hz = 0.5
access = "paid"
base_price_usdc_per_hour = 50  # $0.05/hr in USDC base units (6 decimals = 50 = $0.000050)
# For pricier feeds:
# base_price_usdc_per_hour = 500000  # $0.50/hr

[agent.feeds.eth-gas-trend.sample]
# Sample payload shown to prospective subscribers before they pay
data = '{"ema_12": 42.5, "p25": 35.0, "p75": 55.0, "p95": 120.0, "mev_spike": false}'
```

When the agent boots, `FeedPublisherExt` reads these declarations and registers them with the relay. No manual registration needed.

**Step 2: The FeedPublisherExt extension**

Auto-loaded when `[agent.feeds.*]` entries exist. Handles the full lifecycle: register on boot, publish on each tick, deregister on shutdown.

```rust
pub struct FeedPublisherExt {
    feeds: Vec<FeedConfig>,
    relay: RelayHandle,
}

#[async_trait]
impl Extension for FeedPublisherExt {
    fn name(&self) -> &str { "feed-publisher" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Social }

    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.register_feed(FeedRegistration {
                feed_id: feed.id.clone(),
                agent_id: ctx.agent_id.clone(),
                kind: feed.kind,
                schema: feed.schema.clone(),
                rate_hz: feed.rate_hz,
                access: feed.access.clone(),
                sample: feed.sample.clone(),
            }).await?;
        }
        Ok(())
    }

    async fn on_tick_end(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            if let Some(data) = ctx.cortical.get_feed_data(&feed.id) {
                ctx.relay.publish_feed_data(&feed.id, data).await?;
            }
        }
        Ok(())
    }

    async fn on_shutdown(&mut self, ctx: &mut AgentContext) -> Result<()> {
        for feed in &self.feeds {
            ctx.relay.deregister_feed(&feed.id).await?;
        }
        Ok(())
    }
}
```

**Step 3: Compute the feed data**

The feed's value comes from a `Cognition`-layer extension that runs during the agent's `on_observe` step -- before `FeedPublisherExt` publishes in `on_tick_end`.

```rust
pub struct GasTrendExt {
    ema: f64,
    window: VecDeque<f64>,
}

#[async_trait]
impl Extension for GasTrendExt {
    fn name(&self) -> &str { "gas-trend" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let gas = ctx.cortical.gas_gwei();
        self.window.push_back(gas);
        if self.window.len() > 100 { self.window.pop_front(); }

        // 12-block EMA
        let alpha = 2.0 / 13.0;
        self.ema = alpha * gas + (1.0 - alpha) * self.ema;

        // Percentiles from rolling window
        let mut sorted: Vec<f64> = self.window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p25 = sorted[sorted.len() / 4];
        let p75 = sorted[3 * sorted.len() / 4];
        let p95 = sorted[19 * sorted.len() / 20];

        // MEV spike: gas exceeds 2x the 95th percentile
        let mev_spike = gas > p95 * 2.0;

        ctx.cortical.set_feed_data("eth-gas-trend", json!({
            "ema_12": self.ema,
            "p25": p25,
            "p75": p75,
            "p95": p95,
            "mev_spike": mev_spike,
            "current": gas,
            "ts": now_ms(),
        }));

        Ok(())
    }
}
```

The pipeline order matters: `GasTrendExt` (Cognition layer) runs during `on_observe`, writes data to `cortical`. Then `FeedPublisherExt` (Social layer) runs during `on_tick_end`, reads the data and publishes it to the relay. Extension layers execute in order: Perception -> Cognition -> Social.

**Step 4: Subscribe from a dashboard**

```typescript
// 1. Discover available feeds
const feeds = await fetch(`${relayUrl}/relay/feeds`).then(r => r.json());
const gasFeed = feeds.find(f => f.feed_id === "eth-gas-trend");
// -> { feed_id, agent_id, kind: "derived", rate_hz: 0.5,
//      access: { paid: { price_per_hour: 50 } } }

// 2. Open an MPP session (one-time ERC-3009 signature)
const session = await openMppSession(relayUrl, {
  amount: 500,  // $0.0005 USDC -- enough for ~10 hours at $0.05/hr
  recipient: gasFeed.agent_wallet,
});
// -> { session_id: "abc-123", funded_amount: 500, status: "active" }

// 3. Subscribe to the feed via WebSocket with session auth
const ws = new WebSocket(`${relayUrl}/relay/ws`);
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: "subscribe",
    rooms: [`feed:${gasFeed.feed_id}`],
    payment: {
      intent: "session",
      session_id: session.session_id,
    }
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === "feed_data") {
    // { ema_12: 42.5, p25: 35.0, p75: 55.0, p95: 120.0, mev_spike: false }
    updateGasChart(msg.payload);
  }
  if (msg.type === "payment_draw") {
    // { amount: 1, balance_remaining: 499, session_id: "abc-123" }
    updateBalance(msg.payload);
  }
};
```

**Step 5: Subscribe from another agent**

Agents consume feeds the same way dashboards do, but the subscription is managed by an extension and the session is opened programmatically.

```rust
pub struct GasConsumerExt {
    gas_subscription: Option<FeedSubscription>,
}

#[async_trait]
impl Extension for GasConsumerExt {
    async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let session = ctx.mpp.open_session(
            "gas-oracle",  // agent producing the feed
            500,           // $0.0005 USDC
        ).await?;

        self.gas_subscription = Some(
            ctx.relay.subscribe_feed("eth-gas-trend", session.session_id).await?
        );
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        if let Some(sub) = &self.gas_subscription {
            if let Some(data) = sub.latest() {
                let mev_spike = data["mev_spike"].as_bool().unwrap_or(false);
                if mev_spike {
                    ctx.cortical.set_prediction_error(0.8);
                }
            }
        }
        Ok(())
    }
}
```

### Relay feed infrastructure

The relay manages the feed registry, payment gating, and message forwarding. All feed operations go through the relay -- producers publish to it, subscribers connect through it.

**Feed registration** (agent -> relay on boot):

```json
POST /relay/feeds/register
{
  "feed_id": "eth-gas-trend",
  "agent_id": "gas-oracle",
  "kind": "derived",
  "schema": "gas_trend_v1",
  "description": "12-block EMA gas price with percentile bands and MEV detection",
  "rate_hz": 0.5,
  "access": {
    "paid": {
      "base_price_usdc_per_hour": 50,
      "accepted_protocols": ["x402", "mpp"]
    }
  },
  "sample": {"ema_12": 42.5, "p25": 35.0}
}
```

**Feed discovery** (dashboard or agent -> relay):

```
GET /relay/feeds                              # all feeds
GET /relay/feeds?kind=derived&access=paid     # filter by kind and access
GET /relay/feeds?agent_id=gas-oracle          # feeds from a specific agent
GET /relay/feeds/{feed_id}                    # single feed metadata
GET /relay/feeds/{feed_id}/sample             # sample payload (free, no auth)
```

**Feed subscription with payment** (subscriber -> relay WebSocket):

```json
{
  "type": "subscribe",
  "rooms": ["feed:eth-gas-trend"],
  "payment": {
    "intent": "session",
    "session_id": "abc-123"
  }
}
```

The relay verifies the MPP session with the feed producer's agent, then forwards feed data to the subscriber. Each forwarded message triggers a draw.

**Payment flow through the relay**

```
Subscriber                    Relay                     Feed Producer
    │                           │                            │
    │  Open MPP session         │                            │
    │  (ERC-3009 auth)          │                            │
    │ ────────────────────────> │  Store session ref         │
    │                           │ ─────────────────────────> │
    │  Subscribe to feed room   │                            │
    │  with session_id          │                            │
    │ ────────────────────────> │                            │
    │                           │                            │
    │                           │  <── feed_data ──────────  │
    │                           │                            │
    │                           │  Draw from session:        │
    │                           │  cost = base_price         │
    │                           │        / rate_hz / 3600    │
    │                           │                            │
    │                           │  Draw succeeds?            │
    │  <── feed_data ────────── │  Yes: forward              │
    │  <── payment_draw ─────── │                            │
    │                           │                            │
    │                           │  Draw fails (exhausted)?   │
    │  <── exhaustion_notice ── │  Unsubscribe, notify       │
    │                           │                            │
    │  Top-up session           │                            │
    │ ────────────────────────> │  Resume draws              │
    │                           │                            │
    │  Disconnect / unsubscribe │                            │
    │ ────────────────────────> │  Session stays open        │
    │                           │  (reusable on reconnect)   │
```

### Feed types and composability

Feeds are domain-agnostic. Any agent can produce any feed -- blockchain data, ML model outputs, sentiment analysis, code quality metrics, research signals, market indicators, or arbitrary computed streams. The feed system is the same regardless of domain.

Feeds compose into value chains. Each layer adds computation and charges for it.

**Raw feeds** -- direct data ingestion:

- Blockchain: `eth-mainnet-blocks`, `base-swaps`, `arb-gas` (from RPC WebSocket)
- Research: `arxiv-new-papers`, `github-trending` (from web polling)
- Code: `repo-commit-stream`, `ci-build-results` (from webhooks)
- Market: `binance-funding-rates`, `coingecko-prices` (from exchange APIs)
- Any external data source an agent consumes can be re-published as a raw feed

**Derived feeds** -- computed from raw:

- Blockchain: `eth-gas-trend`, `funding-rate-divergence`, `mev-probability`
- Research: `paper-relevance-scores`, `topic-cluster-updates`
- Code: `code-quality-trend`, `dependency-risk-index`
- Market: `volatility-regime`, `cross-venue-spread`
- Any computation an agent performs on its inputs can be a derived feed

**Composite feeds** -- derived from multiple derived feeds:

- `cross-chain-arb-signal` (consumes gas trends + volume + funding rates)
- `research-portfolio-impact` (consumes paper scores + code quality + market sentiment)
- Cost stacks: producer pays for input feeds, charges for output feed

**Meta feeds** -- feeds about feeds:

- `feed-health` (monitors all feeds for staleness, drift, anomalies)
- `feed-accuracy` (tracks prediction accuracy of derived feeds over time)
- Produced by meta-agents

Composition example:

```
eth-mainnet-blocks (free, raw)
  └─> gas-oracle agent
       └─> eth-gas-trend ($0.05/hr, derived)
            └─> arb-bot agent
                 └─> cross-chain-gas-arb ($0.50/hr, composite)
                      └─> dashboard subscriber

arxiv-new-papers (free, raw)
  └─> research-scout agent
       └─> defi-paper-relevance ($0.02/hr, derived)
            └─> strategy-agent subscribes for research context
```

Each agent in the chain pays for its inputs and charges for its output.

### On-chain feed advertisement (ERC-8004)

Agents with wallets advertise their feeds in their ERC-8004 passport. This makes feeds discoverable on-chain even when the agent or relay is offline.

```solidity
// AgentRegistry.sol — feed advertisement extension
struct FeedAdvert {
    bytes32 feedId;        // keccak256 of feed name
    bytes32 schemaHash;    // keccak256 of schema definition
    uint16  rateMilliHz;   // rate in milli-Hz (500 = 0.5 Hz)
    uint96  pricePerHour;  // USDC base units per hour (0 = free)
    uint32  updatedAt;     // last update timestamp
}

function updateFeeds(FeedAdvert[] calldata adverts) external;
function getFeeds(address agent) external view returns (FeedAdvert[] memory);
```

When an agent boots with feeds configured, it:

1. Registers feeds with the relay (for live presence and subscription routing)
2. Updates its ERC-8004 passport with feed advertisements (for persistent discovery)
3. On feed config changes (add/remove/reprice), updates both relay and chain

```rust
// In FeedPublisherExt::on_boot()
async fn on_boot(&mut self, ctx: &mut AgentContext) -> Result<()> {
    for feed in &self.feeds {
        // Register with relay (live routing)
        ctx.relay.register_feed(/* ... */).await?;

        // Advertise on-chain (persistent discovery)
        if let Some(chain) = &ctx.chain_client {
            chain.update_feed_advert(&ctx.agent_wallet, FeedAdvert {
                feed_id: keccak256(feed.id.as_bytes()),
                schema_hash: keccak256(feed.schema.as_bytes()),
                rate_milli_hz: (feed.rate_hz * 1000.0) as u16,
                price_per_hour: feed.price_usdc_per_hour,
            }).await?;
        }
    }
    Ok(())
}
```

Feed discovery uses both sources:

```typescript
// Dashboard merges relay (live) + chain (persistent) feed data
async function discoverFeeds(): Promise<Feed[]> {
  const [relayFeeds, chainFeeds] = await Promise.all([
    fetch(`${relayUrl}/relay/feeds`).then(r => r.json()),
    chainClient.getRegisteredFeeds(),  // reads all ERC-8004 feed adverts
  ]);

  // Merge: relay has live status, chain has persistent metadata
  return mergeFeeds(relayFeeds, chainFeeds);
  // Result: each feed has { ...chainAdvert, live: boolean, subscribers: number }
}
```

An agent's feeds appear in its passport even when the agent is offline. Users browsing the on-chain registry can see what feeds exist, their pricing, and their schemas -- then subscribe when the agent comes online.

### Dashboard chain feed subscriptions

When the dashboard connects to a blockchain agent, it automatically subscribes to that agent's chain feeds for UI rendering. This is not just data display -- the dashboard uses raw chain feeds to render live blockchain state.

```typescript
// When user opens Agent Detail for a chain agent:
function useAgentChainFeeds(agent: MergedAgent) {
  const feeds = agent.feeds?.filter(f =>
    f.schema.startsWith("eth_") ||
    f.schema.startsWith("evm_") ||
    f.schema === "block" ||
    f.schema === "transaction"
  );

  // Auto-subscribe to chain feeds for live rendering
  for (const feed of feeds ?? []) {
    useRealtimeFeed(feed.feedId, {
      // Chain feeds from a connected agent are rendered as live chain state
      onData: (data) => {
        updateBlockHeight(data.blockNumber);
        updateGasGauge(data.gasUsed);
        updateTransactionList(data.transactions);
      }
    });
  }
}
```

The dashboard renders different UI elements based on feed schema:

| Feed schema | Dashboard renders |
|-------------|-------------------|
| `eth_block` | Block height counter, gas gauge, tx list |
| `evm_logs` | Live event log with contract decode |
| `gas_trend_*` | Gas price sparkline with percentile bands |
| `funding_*` | Funding rate chart with divergence alerts |
| `position_*` | Position cards with P&L |
| `tick_activity_*` | Liquidity heatmap |
| Any custom | Raw JSON viewer with auto-detected chart type |

For non-blockchain feeds, the dashboard renders based on the data shape -- numeric values get sparklines, boolean values get status indicators, arrays get tables.

### Dashboard integration

Three new surfaces: a feed browser, a subscription manager, and a producer revenue panel.

**Feeds page** (in Fleet or System section):

```
+--------------------------------------------------------------+
| Available Feeds                               [+ Publish Feed]|
|                                                               |
| Filter: [All v] [Paid v] [Chain: All v] [Search...]          |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend                                 * LIVE      | |
| | by gas-oracle (Trusted)                      $0.05/hr    | |
| | 12-block EMA gas with percentile bands + MEV detect      | |
| | Schema: gas_trend_v1   Rate: 0.5 Hz   Subs: 7           | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | uniswap-v3-tick-activity                     * LIVE      | |
| | by pool-watcher (Verified)                  $0.20/hr    | |
| | Real-time tick-level activity for top 50 pools           | |
| | Schema: tick_activity_v2   Rate: 2 Hz   Subs: 3         | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-mainnet-blocks                           * LIVE      | |
| | by chain-watcher-1 (Basic)                     FREE     | |
| | Raw Ethereum mainnet block headers                       | |
| | Schema: eth_block   Rate: 0.08 Hz   Subs: 12            | |
| | [Preview]  [Subscribe]                                   | |
| +----------------------------------------------------------+ |
+--------------------------------------------------------------+
```

**My subscriptions** (in Treasury or Settings):

```
+--------------------------------------------------------------+
| My Feed Subscriptions                                         |
|                                                               |
| Active spend: $0.25/hr across 3 feeds                        |
|                                                               |
| +----------------------------------------------------------+ |
| | eth-gas-trend          * Active    Session: $4.82 left    | |
| | gas-oracle             $0.05/hr   Since: 2h ago           | |
| | [Pause] [Top-up $5] [Unsubscribe]                        | |
| +----------------------------------------------------------+ |
|                                                               |
| +----------------------------------------------------------+ |
| | cross-chain-gas-arb    * Active    Session: $1.20 left    | |
| | arb-bot                $0.50/hr   Since: 45m ago          | |
| | [Pause] [Top-up $10] [Unsubscribe]                       | |
| +----------------------------------------------------------+ |
|                                                               |
| Total spent this month: $12.40                                |
| Total earned from my feeds: $8.70                             |
+--------------------------------------------------------------+
```

**Feed detail page** (click into a feed):

```
+--------------------------------------------------------------+
| eth-gas-trend                                     * LIVE      |
| by gas-oracle (Trusted, 342 episodes)            $0.05/hr    |
|                                                               |
| +--------------- Live Preview -------------------------+     |
| | EMA: 42.5 gwei   P25: 35.0   P75: 55.0   P95: 120  |     |
| | MEV: none                                            |     |
| |                                                      |     |
| | [sparkline chart of last 100 data points]            |     |
| +------------------------------------------------------+     |
|                                                               |
| Schema: gas_trend_v1                                          |
| Fields: ema_12 (f64), p25 (f64), p75 (f64), p95 (f64),      |
|         mev_spike (bool), current (f64), ts (u64)            |
|                                                               |
| Uptime: 99.7% (30d)   Avg latency: 120ms                     |
| Subscribers: 7   Revenue: $84.20 (30d)                        |
|                                                               |
| Dependencies: eth-mainnet-blocks (free)                       |
|                                                               |
| Payment: x402 or MPP session                                  |
| [Subscribe with MPP ($5 deposit)]  [Try with x402 ($0.01)]   |
+--------------------------------------------------------------+
```

**Feed revenue** (in Treasury / Cost Analytics):

```
+--------------------------------------------------------------+
| Feed Revenue                                                  |
|                                                               |
| Total earned (30d): $84.20    Active subscribers: 7           |
|                                                               |
| Feed               Subs  Revenue/30d  Status                  |
| eth-gas-trend       7     $84.20      * producing             |
|                                                               |
| [chart: revenue over time, subscriber count over time]        |
|                                                               |
| Settlement: 12 batches settled on-chain                       |
| Pending: $2.30 (next batch in ~8 min)                         |
+--------------------------------------------------------------+
```

### Feed data-source mapping

Extending the page-to-data-source table from the Dashboard architecture section:

| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Fleet / Feeds | `system` | `feed_registered`, `feed_deregistered`, `feed_status` | `GET /relay/feeds` |
| Fleet / Feed detail | `feed:{id}` | `feed_data`, `feed_status`, `payment_draw` | `GET /relay/feeds/{id}` |
| Treasury / Subscriptions | `system` | `session_opened`, `session_exhausted`, `session_settled` | `GET /mpp/sessions` |
| Treasury / Feed Revenue | `system` | `feed_revenue_update`, `settlement_batch` | `GET /relay/feeds/revenue` |

### Practical example: funding rate divergence feed

A second example showing feed composition. This agent consumes two paid feeds and produces a third.

**Agent manifest**

```toml
[agent]
name = "funding-arb"
profile = "chain"
mode = "persistent"

# This agent CONSUMES two feeds...
[[agent.feed_subscriptions]]
feed_id = "binance-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000  # $0.001 USDC session deposit

[[agent.feed_subscriptions]]
feed_id = "hyperliquid-funding-rates"
agent_id = "cex-connector"
budget_usdc = 1000

# ...and PRODUCES one feed
[agent.feeds.funding-divergence]
kind = "derived"
description = "Cross-venue funding rate divergence with z-score normalization"
schema = "funding_divergence_v1"
rate_hz = 0.1  # Every 10 seconds
access = "paid"
base_price_usdc_per_hour = 200000  # $0.20/hr
```

**The extension**

```rust
pub struct FundingDivergenceExt {
    binance_sub: FeedSubscription,
    hyperliquid_sub: FeedSubscription,
    history: VecDeque<f64>,
}

#[async_trait]
impl Extension for FundingDivergenceExt {
    fn name(&self) -> &str { "funding-divergence" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut AgentContext) -> Result<()> {
        let binance = self.binance_sub.latest_or_default();
        let hyper = self.hyperliquid_sub.latest_or_default();

        let divergence = binance["rate"].as_f64().unwrap_or(0.0)
            - hyper["rate"].as_f64().unwrap_or(0.0);

        self.history.push_back(divergence);
        if self.history.len() > 1000 { self.history.pop_front(); }

        let mean = self.history.iter().sum::<f64>() / self.history.len() as f64;
        let variance = self.history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.history.len() as f64;
        let zscore = if variance > 0.0 {
            (divergence - mean) / variance.sqrt()
        } else {
            0.0
        };

        ctx.cortical.set_feed_data("funding-divergence", json!({
            "divergence_bps": divergence * 10000.0,
            "zscore": zscore,
            "binance_rate": binance["rate"],
            "hyperliquid_rate": hyper["rate"],
            "signal": if zscore.abs() > 2.0 { "strong" }
                      else if zscore.abs() > 1.0 { "moderate" }
                      else { "none" },
            "direction": if divergence > 0.0 { "long_hyper" }
                         else { "long_binance" },
            "ts": now_ms(),
        }));

        // Extreme divergence triggers T2 reasoning via prediction error
        if zscore.abs() > 3.0 {
            ctx.cortical.set_prediction_error(0.9);
        }

        Ok(())
    }
}
```

**The value chain**

```
cex-connector produces binance-funding-rates ($0.05/hr)
cex-connector produces hyperliquid-funding-rates ($0.05/hr)
  └─> funding-arb consumes both, pays $0.10/hr
       └─> funding-arb produces funding-divergence ($0.20/hr)
            └─> trading-bot subscribes, pays $0.20/hr
            └─> dashboard subscribes, pays $0.20/hr
```

Economics for `funding-arb`: $0.20/hr revenue per subscriber minus $0.10/hr input cost. With 5 subscribers that's ($0.20 * 5) - $0.10 = $0.90/hr pure margin.

### Extensibility

Any extension can produce a feed. The pattern:

1. Declare the feed in the agent manifest (`[agent.feeds.*]`).
2. Compute data in an extension's `on_observe()` or `on_tick_end()` hook.
3. Store via `ctx.cortical.set_feed_data(feed_id, data)`.
4. `FeedPublisherExt` (auto-loaded when feeds are declared) publishes to the relay.

Users create custom feed schemas, set arbitrary pricing, compose feeds from other feeds. The relay handles discovery, payment gating, and delivery. The dashboard surfaces it all without any feed-specific code -- it reads the schema from the registry and renders accordingly.

---

## Inference gateway

Agents never hold API keys. A centralized `InferenceGateway` inside the roko process owns all secrets, runs every request through a multi-stage pipeline, and calls providers. The gateway is designed as a standalone, reusable system -- it handles caching, cost tracking, loop detection, output budgeting, tool pruning, convergence detection, thinking caps, and batch submission. The `CascadeRouter` from `roko-learn` handles model selection upstream; the gateway handles everything after a model is chosen.

Crate: `crates/roko-gateway/`

### Pipeline overview

Every inference request passes through these stages in order:

```
                              InferenceRequest
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  1. Loop detection   │  Ring buffer of recent tool calls.
                          │     (per-session)    │  Retry / oscillation / drift check.
                          └──────────┬──────────┘
                                     │ pass
                                     ▼
                          ┌─────────────────────┐
                          │  2. Cache lookup     │  L1 hash (blake3) → L2 semantic
                          │     (L1 → L2)       │  (SimHash, Hamming ≤ 3).
                          └──────────┬──────────┘
                               hit / │ miss
                          ┌─────┐    │
                          │return│    │
                          └─────┘    ▼
                          ┌─────────────────────┐
                          │  3. Tool pruning     │  Remove unused tool schemas.
                          │     (per-session)    │  Never prunes core tools.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  4. Output budget    │  EMA-based max_tokens cap.
                          │     (per-model)      │  p95 x 1.5, floor 1024.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  5. Thinking cap     │  Per-model thinking budget.
                          │     (per-model)      │  Only when thinking enabled.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  6. Convergence      │  SimHash of recent responses.
                          │     detection        │  3+ similar → inject guidance.
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  7. Provider call    │  ProviderBackend::complete()
                          │                      │  or ::stream().
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  8. Cache store      │  Write to L1 + L2 (unless
                          │                      │  excluded by cache policy).
                          └──────────┬──────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  9. Cost tracking    │  Compute actual vs naive cost.
                          │                      │  Record per-agent, per-model.
                          └──────────┬──────────┘
                                     │
                                     ▼
                              InferenceResponse
```

### 1. Protocol types

Core types that every subsystem shares.

```rust
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ToolSchema>>,
    pub stream: bool,
    pub thinking: Option<ThinkingConfig>,
    pub metadata: InferenceMeta,
}

pub struct InferenceMeta {
    pub session_id: String,
    pub agent_id: AgentId,
    pub tier: Tier,              // T0, T1, T2
    pub budget_remaining: u64,   // microdollars
}

pub struct InferenceResponse {
    pub text: String,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
    pub model: String,
    pub latency_ms: u64,
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub thinking_tokens: u64,       // Anthropic extended thinking
    pub reasoning_tokens: u64,      // OpenAI reasoning tokens
}

pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    ContentFilter,
}

#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

All types derive `Serialize` + `Deserialize`. `TokenUsage` implements `Add` for aggregation across a session.

### 2. Hash cache (L1)

Exact-match cache. Fast path for repeated identical requests.

**How it works**: Hash the normalized request body with blake3, look up in a moka async LRU cache. If the hash matches, return the cached response without calling a provider.

**Normalization** (applied before hashing):
- Strip UUIDs matching `[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}`
- Strip ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` headers
- Replace git status blocks with `[GIT_STATUS]` placeholder
- Sort JSON keys alphabetically
- Sort tool definitions by name

This ensures that two requests differing only in timestamps or working-directory metadata produce the same hash.

**Cache entry**:

```rust
pub struct CachedResponse {
    pub body: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub cached_at: Instant,
    pub effective_ttl: Duration,
}
```

**Regime-aware TTL**: The system's cortical state controls how long cache entries live.

| Regime | TTL | Rationale |
|--------|-----|-----------|
| Normal | 3600s | Standard operating conditions. |
| Calm | 7200s | Low activity -- cached responses stay valid longer. |
| Volatile | 900s | Rapid changes -- cache expires faster to avoid stale responses. |
| Crisis | 300s | Active failures -- almost no caching, maximize freshness. |

**Exclusions** (never cached):
- Responses containing `tool_use` stop reason (tool call IDs are ephemeral)
- Responses with fewer than 3 output tokens (too short to be useful)
- Error responses

**Storage**: `moka::future::Cache<[u8; 32], CachedResponse>` with configurable max capacity (default 10,000 entries).

### 3. Semantic cache (L2)

Near-miss cache. Catches requests that are semantically equivalent but textually different.

**How it works**: Compute a 64-bit SimHash fingerprint of the request text. Compare against stored fingerprints using Hamming distance. A distance of 3 bits or fewer counts as a cache hit.

**SimHash algorithm**:
1. Tokenize request text (whitespace + punctuation boundaries)
2. Hash each token with a fast 64-bit hash
3. For each bit position: if the token hash has a 1, increment a counter; if 0, decrement
4. Final fingerprint: 1 for each positive counter, 0 for each negative

**Storage**: `DashMap<u64, SimHashEntry>` for lock-free concurrent reads.

```rust
pub struct SimHashEntry {
    pub response: Bytes,
    pub cost_usd: f64,
    pub model: String,
    pub created_at: Instant,
    pub namespace: String,
}
```

**Parameters**:
- Max entries: 5,000
- TTL: 7,200s (fixed, not regime-aware -- semantic matches are fuzzier so the TTL is conservative)
- Eviction: LRU by age when capacity reached
- Hamming threshold: 3 bits (configurable)

**Namespace isolation**: Each tenant/workspace prefixes its cache text with a namespace identifier. This prevents cross-tenant cache hits in multi-user deployments. A `default` namespace is used for single-user setups.

**Exclusions**: Same as L1 -- no tool_use, no sub-3-token, no errors.

### 4. Provider backends and key rotation

Each LLM provider implements a `ProviderBackend` trait:

```rust
#[async_trait]
pub trait ProviderBackend: Send + Sync {
    fn name(&self) -> &str;
    fn supports_model(&self, model: &str) -> bool;
    async fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse>;
    async fn stream(&self, req: &InferenceRequest) -> Result<BoxStream<InferenceChunk>>;
}
```

**Anthropic backend** (`POST https://api.anthropic.com/v1/messages`):
- Streaming via SSE
- Tool use with full schema
- Extended thinking (`thinking.type = "enabled"`, `thinking.budget_tokens`)
- Prefix caching: system block annotated with `cache_control: {"type": "ephemeral", "ttl": "1h"}`
- Extracts `cache_read_input_tokens`, `cache_creation_input_tokens`, `thinking_tokens` from response usage

**OpenAI backend** (`POST https://api.openai.com/v1/chat/completions`):
- Format translation: Anthropic message format <-> OpenAI chat format
- Reasoning token extraction from `prompt_tokens_details.cached_tokens` and `completion_tokens_details.reasoning_tokens`
- Model routing: handles `gpt-*`, `o1`, `o3-*`, `o4-*`

**Key rotation**: Each provider holds a `Vec<String>` of API keys. On a 429 (rate limit) response, the provider rotates to the next key in the list. An `AtomicUsize` index tracks the active key. Rotation is lock-free.

```rust
pub struct KeyRing {
    keys: Vec<String>,
    active: AtomicUsize,
}

impl KeyRing {
    pub fn current(&self) -> &str {
        let idx = self.active.load(Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    pub fn rotate(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }
}
```

**Provider resolution order**: Anthropic for `claude-*` models, OpenAI for `gpt-*/o1/o3-*/o4-*`. Additional providers (Gemini, Perplexity, Ollama, OpenRouter) use the existing `roko-agent` backends and are registered by config.

### 5. Cost computation

Per-request cost calculation with actual vs naive pricing comparison.

**Pricing table**: `HashMap<String, ModelPricing>` loaded from config. Supports substring matching for model families (e.g., `claude-sonnet` matches `claude-sonnet-4-20250514`).

```rust
pub struct ModelPricing {
    pub input_per_m: f64,          // USD per 1M input tokens
    pub output_per_m: f64,         // USD per 1M output tokens
    pub cached_input_per_m: f64,   // USD per 1M cached input tokens
    pub reasoning_per_m: f64,      // USD per 1M reasoning/thinking tokens
}
```

Default fallback: $3/M input, $15/M output (covers unknown models without crashing).

**Cost formula** (per request):

```
fresh_input   = (input_tokens - cache_read_tokens) * input_per_m / 1e6
cached_input  = cache_read_tokens * cached_input_per_m / 1e6
cache_write   = cache_creation_tokens * input_per_m * 1.25 / 1e6    # 25% surcharge
regular_out   = (output_tokens - reasoning_tokens) * output_per_m / 1e6
reasoning     = reasoning_tokens * reasoning_per_m / 1e6
thinking      = thinking_tokens * output_per_m / 1e6

actual_cost   = fresh_input + cached_input + cache_write + regular_out + reasoning + thinking
```

**Batch discount**: Requests submitted through the batch API get a 50% reduction on `actual_cost`.

**Naive cost**: What the provider would charge with no caching at all:

```
naive_cost = total_input_tokens * input_per_m / 1e6  +  total_output_tokens * output_per_m / 1e6
```

**Savings**: `naive_cost - actual_cost`. Tracked per request and aggregated per agent, per session, and per model for dashboard display.

**Attribution**: Every cost record includes `agent_id` and `session_id`. This feeds the Treasury / Cost page in the dashboard and the per-agent cost breakdowns.

### 6. Loop detection

Detects three patterns of agent loops and injects corrective guidance before the agent wastes more tokens.

**Per-session state**:

```rust
pub struct SessionLoopState {
    recent_calls: VecDeque<(String, [u8; 32])>,  // (tool_name, blake3(args))
    consecutive_identical: u32,
    tokens_since_progress: u64,
}
```

Ring buffer capacity: 16 entries. Does not grow.

**Detection rules**:

| Pattern | Trigger | Injected guidance |
|---------|---------|-------------------|
| Retry | Same tool + same args hash called 5+ times consecutively | "You have called the same tool with the same arguments 5 times. Try a different approach." |
| Oscillation | A -> B -> A -> B pattern repeats 3+ full cycles | "You are oscillating between two actions. Break the loop by choosing a third option or stopping." |
| Drift | 15,000+ output tokens accumulated without new `tool_result` content | "You have generated 15K+ tokens without making progress. Either take a concrete action or stop." |

**Injection mechanism**: The guidance string is prepended to the system prompt on the next request. It appears once and clears itself.

**Counters**: `loops_detected`, `loop_injections`, `loop_retry_detected`, `loop_oscillation_detected`, `loop_drift_detected`. All exposed via the stats endpoint.

### 7. Output budgeting

Prevents runaway output by auto-setting `max_tokens` based on observed behavior.

**Per-model tracking**:

```rust
pub struct ModelOutputStats {
    pub ema: f64,           // exponential moving average of output tokens
    pub ema_sq: f64,        // EMA of squared output tokens (for variance)
    pub max_seen: u64,      // highest output observed
    pub count: u64,         // total observations
}
```

**Algorithm**:
- Alpha: 0.05 (5% weight to new observations)
- Minimum samples: 20 before p95 estimation is trusted
- p95 estimate: `ema + 2 * sqrt(ema_sq - ema^2)` (EMA + 2 standard deviations)
- Cap: `p95 * 1.5`, with a floor of 1,024 tokens

**Behavior**:
- When a request has no `max_tokens` set, the gateway auto-sets it to the computed cap
- When a request has an unreasonably high `max_tokens` (above 2x the cap), the gateway reduces it to the cap
- When a request has an explicit `max_tokens` that is *below* the cap, the gateway does not touch it

**Counters**: `output_budgets_applied`, `output_tokens_bounded`.

### 8. Tool pruning

Removes unused tool schemas from requests to reduce input token count. Tool schemas are verbose (often 200-500 tokens each), and most sessions use a small subset.

**Usage tracking**: Two maps:
- Per-session: `HashMap<String, u32>` -- how many times each tool was called in this session
- Global: `HashMap<String, u64>` -- how many times each tool has been called across all sessions

**Never-prune list** (core tools that must always be available):
`Bash`, `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebSearch`, `WebFetch`, `TaskCreate`, `TaskUpdate`, `TaskList`, `Agent`, `SendMessage`

**Two-tier pruning**:

| Tier | Trigger | Logic |
|------|---------|-------|
| Session (Tier 1) | 50+ requests in the current session | Remove tools never used in this session. Protected + used tools survive. |
| Global (Tier 2) | < 50 session requests but 50+ total global requests | Remove tools never used by any session. Catches tools that are defined but universally ignored. |

**Metrics**: `tools_pruned` count, `tool_tokens_saved` estimate (removed schemas x average schema size of ~300 tokens).

### 9. Convergence detection

Detects when an agent is producing repetitive responses and needs a nudge.

**Per-session state**:

```rust
pub struct ConvergenceState {
    recent_hashes: VecDeque<u64>,  // last 8 response SimHashes
    consecutive_similar: u32,
}
```

**Detection**: After each response, compute its SimHash. Compare to the previous response's SimHash via Hamming distance. If the distance is 2 bits or fewer, increment `consecutive_similar`. Three or more consecutive similar responses triggers convergence.

**Injection**: On the next request, prepend: "Your recent responses are converging. Try a different angle or move to the next step."

A dissimilar response (Hamming > 2) resets the counter to zero.

**Counters**: `convergence_detected`, `convergence_injections`.

### 10. Thinking cap

Per-model defaults for extended thinking budgets. Prevents agents from using unbounded thinking tokens when the budget is unset.

| Model family | Default thinking budget |
|-------------|------------------------|
| Opus | 32,768 tokens |
| Sonnet | 16,384 tokens |
| Haiku | 4,096 tokens |

**Rules**:
- Activates only when thinking is already enabled (`thinking.type = "enabled"`) but `budget_tokens` is absent
- Never forces thinking on. If thinking is disabled, the cap does nothing.
- Never overrides explicit user budgets. If the user sets `budget_tokens: 8192`, the cap does not increase it.

**Counters**: `thinking_budgets_applied`, `thinking_tokens_capped_estimate`.

### 11. Batch API

Queues inference requests for asynchronous batch processing at a 50% cost discount. Useful for non-time-sensitive work: plan generation, research, code review.

**Queue behavior**:
- Requests submitted via `POST /api/gateway/batch/submit` return `202 Accepted` with a `custom_id` (`roko-{uuid}`)
- Auto-flush triggers: 50 items accumulated OR 30 seconds elapsed
- Manual flush: `POST /api/gateway/batch/flush`

**Submission**: On flush, the gateway submits the batch to `POST https://api.anthropic.com/v1/messages/batches`.

**Polling**: Background task polls `GET /v1/messages/batches/{batch_id}` every 60 seconds until the batch completes.

**Results**: Stored in `DashMap<String, BatchResult>` keyed by `custom_id`. Retrieved via `GET /api/gateway/batch/result/{custom_id}`.

**Preprocessing**: Batch requests go through the same pipeline stages as real-time requests (prefix caching, output budget, tool pruning). Cost calculation applies the 50% batch discount.

### Gateway HTTP routes

```
POST   /api/gateway/inference         Main inference proxy endpoint.
                                       Auth required (agent token).
                                       Runs full pipeline.
                                       Returns InferenceResponse.

GET    /api/gateway/stats             Aggregate gateway statistics:
                                       cache hit rates, total cost,
                                       active sessions, loop detections,
                                       convergence events, tool pruning savings.

GET    /api/gateway/ws                WebSocket endpoint streaming per-request
                                       StatsEvents in real time.
                                       Broadcast channel (1024 slot capacity).

POST   /api/gateway/batch/submit      Queue a request for batch processing.
                                       Returns 202 + custom_id.

POST   /api/gateway/batch/flush       Force-flush the current batch queue.

GET    /api/gateway/batch/result/:id  Retrieve completed batch result by
                                       custom_id.
```

**StatsEvent** (broadcast on the WebSocket per completed request):

```rust
pub struct StatsEvent {
    pub seq: u64,
    pub timestamp_ms: u64,
    pub model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
    pub naive_cost_usd: f64,
    pub savings_usd: f64,
    pub cache_hit: bool,
    pub elapsed_ms: u64,
    pub session_id: String,
    pub gateway_actions: Vec<String>,  // e.g., ["output_budget", "tool_prune"]
}
```

### InferenceHandle

In-process agents get an `InferenceHandle` -- a channel sender that communicates with the gateway without holding any secrets.

```rust
/// Handle given to agents for inference requests.
/// Contains no API keys -- only a channel sender.
#[derive(Clone)]
pub struct InferenceHandle {
    sender: mpsc::Sender<InferenceRequest>,
    agent_id: AgentId,
    budget: Arc<AtomicU64>,  // remaining budget in microdollars
}

impl InferenceHandle {
    /// Send an inference request and await the response.
    pub async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to: tx,
        }).await?;
        rx.await?
    }

    /// Stream an inference response (for LLM output).
    pub async fn infer_stream(
        &self,
        request: InferenceRequest,
    ) -> Result<impl Stream<Item = InferenceChunk>> {
        let (tx, rx) = mpsc::channel(64);
        self.sender.send(InferenceEnvelope {
            agent_id: self.agent_id.clone(),
            request,
            respond_to_stream: tx,
        }).await?;
        Ok(ReceiverStream::new(rx))
    }

    /// Remaining budget in microdollars.
    pub fn remaining_budget(&self) -> u64 {
        self.budget.load(Ordering::Relaxed)
    }
}
```

### CascadeRouter integration

The gateway uses the existing `CascadeRouter` from `roko-learn` for model selection. The router picks the model; the gateway handles everything after that.

```rust
impl InferenceGateway {
    async fn route_request(&self, envelope: InferenceEnvelope) -> Result<()> {
        // 1. Select model via CascadeRouter
        let model = self.cascade_router.select_model(
            &envelope.request.task_type,
            envelope.request.tier,
            &envelope.agent_id,
        );

        // 2. Stamp model onto request
        let mut request = envelope.request;
        request.model = model.clone();

        // 3. Run through gateway pipeline
        //    loop_check -> cache_lookup -> tool_prune -> output_budget
        //    -> thinking_cap -> convergence_check -> provider_call
        //    -> cache_store -> cost_track
        let response = self.pipeline.execute(request).await?;

        // 4. Update router weights from quality signal
        self.cascade_router.record_outcome(
            &model,
            &envelope.request.task_type,
            &response.quality_signal,
        );

        // 5. Publish cost update to relay
        self.relay.publish_cost_update(
            &envelope.agent_id,
            response.usage.total_cost_microdollars,
        ).await;

        envelope.respond(response);
        Ok(())
    }
}
```

### Proxying for isolated agents

Remote agents (Fly Machines, Railway containers) don't have direct access to the inference gateway's channel. They make HTTPS requests to the parent's proxy endpoint:

```
POST /api/inference/proxy
Authorization: Bearer <agent_token>
Content-Type: application/json

{
  "agent_id": "isolated-coder-1",
  "model_hint": "auto",
  "tier": "t1",
  "messages": [ ... ],
  "tools": [ ... ],
  "max_tokens": 4096
}
```

The proxy endpoint validates the agent token, deducts from the agent's budget, and forwards the request through the same gateway pipeline. The agent never sees API keys.

---

## Authentication

Four auth paths for four surfaces.

### 1. Dashboard users: Privy

```
Browser → Privy SDK → JWT → roko-serve validates signature
```

Privy handles login (email, social, wallet). The dashboard includes the JWT in every API call. roko-serve validates the JWT signature against Privy's JWKS endpoint.

```
GET https://auth.privy.io/.well-known/jwks.json
→ Cache JWKS, verify JWT signature + expiry
→ Extract: sub (privy user ID), email, wallet address
→ Lookup or create user in .roko/users/
```

Privy also provides an embedded wallet for chain interactions (signing transactions, delegating to agents). Optional -- users who don't need chain features never see wallet UI.

### 2. CLI: API keys + roko login

```bash
# Generate an API key (from dashboard or CLI)
roko config secrets set api.my-key
# → sk_roko_aBcDeFgHiJkLmNoP

# Use it
roko status --server https://my-roko.up.railway.app --api-key sk_roko_...

# Or: roko login (browser-based)
roko login https://my-roko.up.railway.app
# → Opens browser for Privy auth
# → Stores session token in OS keychain

# On headless machines: device flow
roko login https://my-roko.up.railway.app
# → Visit https://my-roko.up.railway.app/auth/device
#   Enter code: ABCD-EFGH
# → Polls until approved
# → Token stored in OS keychain
```

API keys have scopes:

```rust
pub enum ApiKeyScope {
    Read,        // GET endpoints only
    AgentWrite,  // Agent CRUD + messaging
    PlanWrite,   // Plan/PRD creation and execution
    Admin,       // Everything including secrets and config
}
```

### 3. Agent auth: bearer tokens

Agents authenticate to the relay and to the inference proxy using bearer tokens issued by the control plane.

```bash
# Control plane issues token for an agent
POST /api/agents/:id/token
→ { "token": "roko_agent_...", "expires_at": "2026-04-25T00:00:00Z" }

# Agent uses token for relay connection
WS wss://relay.nunchi.dev/relay/ws
→ First message: { "type": "auth", "token": "roko_agent_..." }

# Agent uses token for inference proxy
POST /api/inference/proxy
Authorization: Bearer roko_agent_...
```

Tokens are SHA-256 hashed before storage. The plaintext is returned exactly once at issuance.

### 4. Relay auth: reads public, writes authenticated

```
Read operations (subscribe, list feeds):    No auth required
Write operations (publish, register feed):  Require agent token
Admin operations (force-disconnect):        Require API key with admin scope
```

This means the dashboard can subscribe to presence and feeds without authentication. It needs auth only to send messages to agents or modify configuration.

### Agent-to-agent auth for paid feeds

Paid feed subscriptions use the same agent token mechanism. The subscribing agent's token is validated by the relay, and payment is recorded against the agent's budget.

---

## Secret and API key management

### Storage hierarchy

```
Priority    Source              Where
────────    ──────              ─────
1 (highest) Environment vars   ANTHROPIC_API_KEY, PERPLEXITY_API_KEY, etc.
2           Secrets store       .roko/secrets.toml (encrypted at rest)
3           Config file         roko.toml [providers] section (not recommended)
```

### Secrets store format

```toml
# .roko/secrets.toml
# Encrypted with age (https://age-encryption.org)
# Key derived from machine identity or user passphrase

[llm]
anthropic = "sk-ant-..."
perplexity = "pplx-..."
gemini = "AIza..."
openrouter = "sk-or-..."
moonshot = "sk-..."
zai = "..."

[integration]
github = "ghp_..."
slack = "xoxb-..."

[infra]
fly_api_token = "fo1_..."
railway_token = "..."
```

### From the CLI

```bash
# Set a secret (reads from stdin, never in shell history)
echo "sk-ant-xyz" | roko config secrets set llm.anthropic

# Interactive prompt
roko config secrets set llm.anthropic
# Enter secret: ****

# List configured secrets (keys, never values)
roko config secrets list
# NAMESPACE    KEY          SOURCE        STATUS
# llm          anthropic    secrets.toml  * valid
# llm          perplexity   env var       * valid
# integration  github       secrets.toml  * valid
# llm          gemini       --            o not set

# Validate all secrets
roko config check-secrets
# Anthropic: valid (claude-sonnet-4-6 accessible)
# Perplexity: valid
# GitHub: valid (repo scope, expires 2026-06-01)
# Gemini: not configured
```

### From the dashboard

Settings > Provider Keys page. Each provider shows a status indicator (connected / not set / invalid). Users paste keys into a form. The dashboard sends them to `POST /api/secrets/:ns/:key`.

Test button calls `POST /api/secrets/:ns/:key/test` -- the server makes a minimal API call to the provider and returns connection status.

**Client-only mode**: keys stored in `localStorage`, sent via `X-Provider-Keys` header per request. The server uses them but never persists them.

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

---

## Dashboard architecture

The dashboard works in two modes:

- **Backbone only**: connected to relay. Shows agent presence, chain data, feeds. No plans, PRDs, or learning -- those require roko-serve.
- **Full mode**: connected to relay AND roko-serve. Shows everything.

The UI gracefully degrades. When roko-serve is unreachable, workspace tabs (Plans, PRDs, Learning) show "Connect to a roko workspace to use this feature" instead of an error.

### Data layer

Three components manage the flow from WebSocket events to rendered pixels.

**SubscriptionManager**: Multiplexes connections to agent, chain, relay, and workspace event streams. Each dashboard page declares its subscriptions on mount and releases them on unmount. The manager maintains a single WebSocket to the relay and a single WebSocket to roko-serve, using room-based subscription messages to filter server-side.

```typescript
// Per-page lifecycle
function useDashboardSubscriptions(rooms: string[]) {
  const manager = useSubscriptionManager();

  useEffect(() => {
    manager.subscribe(rooms);
    return () => manager.unsubscribe(rooms);
  }, [rooms]);
}
```

**EventAggregator**: Batches burst events with a 100ms flush window. High-frequency sources (heartbeats at 100ms, chain blocks at 2s) produce more events than the DOM can absorb per frame. The aggregator collects events during the flush window and delivers them as a single batch. A ring buffer (200 events) supports replay for components that mount after events have already fired.

**RenderScheduler**: Coordinates DOM and canvas updates. DOM updates are coalesced and applied in rAF callbacks. Canvas/WebGL renders (Three.js visualizations, real-time charts) run at 60fps on a separate requestAnimationFrame loop. The scheduler prevents DOM thrashing by batching state changes from the EventAggregator into single React renders.

**Three-tier motion system**:

| Tier | Source | Visual expression |
|------|--------|-------------------|
| Heartbeat rhythm | Per-agent heartbeat ticks (100ms-1s) | Agent card pulse, glow intensity |
| Event-paced tickers | Chain blocks (~2s), gate results, task completions | Counter increments, progress bar steps |
| Ambient decay | Knowledge staleness (Ebbinghaus curve), feed inactivity | Fade, desaturation, visual aging |

### Page-to-data-source mapping

Every dashboard section subscribes to specific WebSocket rooms and event types. REST fallbacks provide initial state on page load when WebSocket history is insufficient.

| Section | WS rooms | Event types | REST fallback |
|---------|----------|-------------|---------------|
| Pulse / Command center | `system`, `agent:*:heartbeat` | `heartbeat_aggregate`, `agent_status`, `presence_join`, `presence_leave` | `GET /relay/api/agents` |
| Pulse / Live console | `agent:*`, `agent:*:heartbeat` | `heartbeat`, `output_chunk`, `gate_result` | -- |
| Pulse / Event stream | `system`, `agent:*` | All event types (filtered client-side by user selection) | -- |
| Fleet / Agent fleet | `system` | `presence_join`, `presence_leave`, `heartbeat_aggregate` | `GET /relay/api/agents` |
| Fleet / Agent detail | `agent:{id}`, `agent:{id}:heartbeat`, `agent:{id}:output`, `agent:{id}:trace` | `heartbeat`, `output_chunk`, `gate_result`, `trace`, `cost_update` | -- |
| Forge / Plans | `plan:*` | `task_started`, `task_completed`, `phase_transition` | `GET /api/plans` |
| Forge / Execution | `plan:*`, `agent:*` | `task_started`, `task_completed`, `gate_result`, `output_chunk` | `GET /api/plans/:id` |
| Knowledge / Store | `chain:knowledge` | `knowledge_published`, `knowledge_validated`, `knowledge_challenged` | `GET /mirage/api/knowledge` |
| Knowledge / Stigmergy | `chain:pheromones` | `pheromone_deposited`, `pheromone_decayed` | `GET /mirage/api/pheromones` |
| Treasury / ISFR | `chain:isfr` | `isfr_updated`, `rate_changed` | `GET /mirage/api/isfr` |
| Treasury / Positions | `agent:{id}` | `position_opened`, `position_closed`, `pnl_update` | -- |
| Treasury / Cost | `system` | `cost_update`, per-request from gateway WS | `GET /api/gateway/stats` |
| Arena / Leaderboard | `arena:{id}` | `attempt_completed`, `score_updated` | `GET /api/arenas/:id` |
| System / Providers | `system` | `provider_status`, per-request from gateway WS | `GET /api/gateway/stats` |
| System / Jobs | `system` | `job_created`, `job_assigned`, `job_completed` | `GET /api/jobs` |

### Chain feed integration

The dashboard consumes agent-exposed chain feeds defined in the "Blockchain agent feeds" section of this document.

**Agent list**: Each agent's card shows its registered feeds (from the relay feed registry). A chain agent with three feeds shows three small feed indicators with live rates.

**Agent detail page**: When viewing a single agent, its active chain feeds are displayed as live data panels -- raw RPC data, derived indicators, signals, and analysis. This is the same data the agent is processing, made visible to the operator.

**Treasury pages**: Raw and derived price/rate feeds from chain agents flow directly into Treasury views. ISFR rates, position P&L, and cost data all arrive via WebSocket subscription. No polling.

**Feed subscription rule**: All chain data flows through the relay's WebSocket room system (`agent:{id}:feed:{feed_id}`). The dashboard subscribes on page mount and unsubscribes on unmount. Feeds are never polled.

### Adaptive information density

The dashboard adjusts its information density based on the system's `CorticalState`. Three display regimes:

```
Regime      Trigger                     What changes
──────      ───────                     ────────────
Cruise      All agents calm,            Minimal display. Green status dots.
(calm)      no active plans,            Aggregated metrics only. Agent cards
            PE < 0.15 avg               collapsed to single-line summaries.

Volatile    1+ agents in T2,            Affected agents expand automatically.
            active gate failures,       Healthy agents stay collapsed.
            PE 0.15-0.40 avg            Event stream highlights anomalies.

Crisis      Multiple gate failures,     Full traces visible. Remediation
            agent errors,               suggestions shown inline. Per-tick
            PE > 0.40 avg               timeline appears. All agents expand.
```

The regime transitions smoothly (CSS transitions, not hard cuts). The dashboard computes the regime from the aggregate cortical state of all connected agents, updated on each heartbeat aggregate event.

### Progressive disclosure

Three interaction layers control how much detail is visible.

**Layer 0 -- Summary** (visible without interaction):
```
12 agents online    3 plans active    $4.23/hr burn    99.2% gate pass rate
```
One line. Scannable in under a second.

**Layer 1 -- Detail** (one click/hover):
```
Per-agent costs:    coder-1 $0.02/hr  |  research $0.08/hr  |  coder-2 $0.15/hr
Per-model split:    Sonnet 72%  |  Haiku 24%  |  Opus 4%
Cache savings:      $12.40 saved today (L1: 68%, L2: 12%)
```
Breakdown appears in a popover or expanded card.

**Layer 2 -- Trace** (second interaction from Layer 1):
```
Full token log, diff view, per-request cost, HDC fingerprint, gate rung detail,
tool call history, convergence/loop detection events, thinking token breakdown
```
Opens in a slide-out panel or dedicated sub-page.

### Epistemic aesthetics

Visual properties react to system state. These are not decorative -- each visual channel encodes a data dimension.

| Visual property | Data source | Encoding |
|----------------|-------------|----------|
| Glow intensity | Epistemic confidence (gate pass rate, neuro store match quality) | Brighter = higher confidence in agent's knowledge |
| Fade / decay | Knowledge staleness (Ebbinghaus forgetting curve) | Faded entries need re-validation or consolidation |
| Turbulence | Contested knowledge entries (challenged in neuro store) | Shimmering/jittering indicates active dispute |
| Velocity streaks | Active agent output (tokens/sec) | Faster streaks = higher throughput |
| Heartbeat pulse | Per-agent tick cadence (gamma/theta/delta) | Visible rhythm matches agent's adaptive clock |
| Saturation | Validation strength (gate rung depth) | Deeper validation = richer color |

Implemented via Three.js shaders for canvas elements and CSS custom properties for DOM elements. The `CorticalState` broadcast drives all six channels.

### Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Nunchi              [+ Agent]  [+ Cluster]  [user@email v]      │
├──────┬──────────────────────────────────────────────────────────┤
│      │                                                          │
│ Nav  │  Overview                                                │
│      │  ┌────────────────────────────────────────────────────┐  │
│ Home │  │ * 5 agents   2 clusters   $4.23 today   ^ 3d 2h   │  │
│      │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│Agents│  Agents                                                  │
│      │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│      │  │ coder-1  │ │ research │ │ chain-1  │ │ coder-2  │   │
│Feeds │  │ * T0     │ │ * T1     │ │ * T0     │ │ o T2     │   │
│      │  │ coding   │ │ research │ │ chain    │ │ coding   │   │
│      │  │ idle     │ │ querying │ │ monitor  │ │ building │   │
│Plans │  │ $0.02/hr │ │ $0.08/hr │ │ $0/hr    │ │ $0.15/hr │   │
│      │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│      │                                                          │
│Learn │  Cluster: feature-xyz                                    │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ researcher --> impl-1 --> reviewer                 │  │
│Costs │  │ + done        o working   . waiting                │  │
│      │  │               impl-2 --/                           │  │
│      │  │               o working                            │  │
│ Logs │  └────────────────────────────────────────────────────┘  │
│      │                                                          │
│  ⚙   │  Agent: coder-2 (expanded)                              │
│      │  ┌────────────────────────────────────────────────────┐  │
│      │  │ Status: T2 reasoning  |  Uptime: 12m  |  Cost: $1.8│  │
│      │  │ Task: Implement pagination in users API            │  │
│      │  │                                                    │  │
│      │  │ Heartbeat ─────────────────────────────            │  │
│      │  │ T0 T0 T0 T0 T1 T0 T0 T2 T0 T0 T0 T1 [T2]       │  │
│      │  │                                                    │  │
│      │  │ Logs (live -- WebSocket, not polling)              │  │
│      │  │ 14:32:21 [T2] PE=0.73 -> full reasoning           │  │
│      │  │ 14:32:25 [T2] action: edit src/users.rs:142       │  │
│      │  │ 14:32:30 [T0] verify: cargo test -> 47 passed     │  │
│      │  │                                                    │  │
│      │  │ [Stop] [Restart] [View Full Trace] [Open in CLI]  │  │
│      │  └────────────────────────────────────────────────────┘  │
└──────┴──────────────────────────────────────────────────────────┘
```

### Feeds page

A dedicated page for browsing agent chain feeds:

```
┌──────────────────────────────────────────────────────────────────┐
│ Feeds                                                            │
│                                                                  │
│ Active feeds (6)                              [Subscribe to new] │
│                                                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ eth-mainnet-blocks    chain-watcher-1    raw     0.08 Hz    │ │
│ │ public | 3 subscribers                                       │ │
│ │ Latest: block 21,432,891 | 142 txs | 15.2 gwei             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ eth-gas-trend         chain-watcher-1    derived  0.5 Hz    │ │
│ │ paid ($1/hr) | 1 subscriber                                 │ │
│ │ Latest: trend=rising, 5m_avg=18.4, 1h_avg=15.7             │ │
│ ├──────────────────────────────────────────────────────────────┤ │
│ │ base-dex-swaps        defi-scanner      derived  2.0 Hz    │ │
│ │ public | 5 subscribers                                       │ │
│ │ Latest: WETH/USDC swap 12.5 ETH @ $3,241.50                │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Feed detail: eth-mainnet-blocks                                  │
│ ┌──────────────────────────────────────────────────────────────┐ │
│ │ [live chart: block times + gas prices, last 100 blocks]     │ │
│ │                                                              │ │
│ │ Recent events:                                               │ │
│ │ 14:32:45  block 21,432,891  142 txs  15.2 gwei  0.8s       │ │
│ │ 14:32:33  block 21,432,890  98 txs   14.8 gwei  12.1s      │ │
│ │ 14:32:21  block 21,432,889  203 txs  16.1 gwei  12.0s      │ │
│ └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### Settings page

```
Settings
|-- Account
|   |-- Profile (name, email, avatar from Privy)
|   +-- Wallet (Privy embedded wallet address, delegation status)
|
|-- Provider keys
|   |-- Anthropic (Claude) ---- * connected
|   |-- Perplexity (Sonar) ---- o not set
|   |-- Google (Gemini) ------- o not set
|   |-- Moonshot (Kimi) ------- o not set
|   |-- ZAI (GLM) ------------ o not set
|   |-- OpenRouter ----------- o not set
|   |-- Ollama --------------- * found (localhost:11434)
|   +-- Storage: [server-side v] / [client-only]
|
|-- Integrations
|   |-- GitHub -- o not set (enables PR creation)
|   |-- Slack --- o not set (enables notifications)
|   +-- Railway - * connected (OAuth)
|
|-- Infrastructure
|   |-- Fly.io -- o not set (enables isolated agents)
|   |-- Control plane: https://my-roko.up.railway.app -- * healthy
|   |-- Relay: wss://relay.nunchi.dev -- * healthy
|   +-- Mirage: https://mirage-devnet.fly.dev -- * healthy
|
|-- API keys
|   |-- github-actions (agent:write) -- created 2d ago
|   |-- [+ Create key]
|   +-- [Manage keys]
|
+-- Team (phase 2)
    |-- Members
    |-- Invitations
    +-- Roles
```

### Tech stack

| Layer | Library | Version | Purpose |
|-------|---------|---------|---------|
| Framework | React | 19 | Component model, concurrent features |
| Build | Vite | 8 | Dev server, production bundling |
| Data fetching | TanStack Query | 5 | REST cache, stale-while-revalidate |
| State | Zustand | 5 | Client-side stores (agent state, UI preferences) |
| Blockchain | ethers.js | 6 | Chain reads, contract interaction |
| 3D / Canvas | Three.js | latest | Epistemic visualizations, particle systems |
| Charts | Recharts | latest | Time series, cost breakdowns, gate pass rates |
| Auth | Privy | 3 | Wallet + social login, embedded wallets |

### Performance targets

| Metric | Target | How |
|--------|--------|-----|
| FCP | < 1.2s | Code splitting per route, preloaded critical CSS |
| LCP | < 2.0s | SSR for initial state, streaming HTML |
| CLS | < 0.05 | Reserved layout slots for async data |
| WS event-to-render p95 | < 100ms | EventAggregator batching + rAF scheduling |
| Canvas/WebGL | >= 60fps sustained | Separate render loop, instanced geometry |
| Initial JS bundle | < 250KB gzipped | Tree shaking, dynamic imports for heavy deps (Three.js, ethers) |

---

## API surface

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
POST   /api/agents/:id/token         Issue/rotate agent bearer token
GET    /api/agents/:id/token/status   Token status (exists, expiry)
GET    /api/agents/:id/feeds          List feeds exposed by this agent
```

### Clusters

```
POST   /api/clusters                  Create cluster with pipeline
GET    /api/clusters                  List clusters
GET    /api/clusters/:id              Cluster status (pipeline progress)
POST   /api/clusters/:id/stop         Stop all agents in cluster
DELETE /api/clusters/:id              Destroy cluster + all agents
```

### Inference gateway

```
POST   /api/inference/proxy           Proxied inference for remote agents
GET    /api/inference/stats           Gateway stats (cache hit rate, costs, latency)
GET    /api/inference/models          Available models + routing weights
POST   /api/inference/models/:id/pin  Pin a model for an agent (override router)
```

### Feeds

```
GET    /api/feeds                     List all registered feeds
GET    /api/feeds/:feed_id            Feed detail + recent data
POST   /api/feeds/:feed_id/subscribe  Subscribe to a feed
DELETE /api/feeds/:feed_id/subscribe  Unsubscribe from a feed
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

### WebSocket

```
WS     /ws                            roko-serve event stream (plans, gates, episodes)
WS     /relay/ws                      Relay event stream (presence, feeds, messages)
```

Both WebSocket endpoints support the room-based subscription protocol described in "Data flow: subscription-only."

### Infrastructure

```
GET    /api/health                     Service health
GET    /api/providers                  Configured LLM providers + health
GET    /api/costs                      Cost summary (per agent, per day, per model)
GET    /api/costs/:agent_id            Cost breakdown for one agent
GET    /api/relay/health               Relay connection health
```

### Existing routes

All ~85 existing routes in roko-serve (plans, PRDs, gates, episodes, signals, knowledge, learning, research, diagnosis, deployments, etc.) remain unchanged. They get auth middleware added.

---

## Configuration

### roko.toml -- full example

```toml
# Workspace-level configuration

[server]
bind = "0.0.0.0"
port = 6677

[serve]
port = 6677

[serve.auth]
enabled = true
# api_key is set via secrets store, not in config

[relay]
url = "wss://relay.nunchi.dev"
# Agent auto-registers with relay on startup

[inference]
# Default model routing (overridable per-agent)
gamma_model = "claude-haiku-4-5"
theta_model = "claude-sonnet-4-6"
delta_model = "claude-opus-4-6"

# Cache settings
cache_l1_ttl_secs = 300      # Exact match cache
cache_l2_ttl_secs = 1800     # Semantic similarity cache
cache_l2_threshold = 0.95    # Minimum similarity for L2 hit

# Cost limits
daily_budget_usd = 50.0
per_agent_daily_usd = 10.0

[chain]
rpc_url = "https://mirage-devnet.fly.dev"
chain_id = 31337
# wallet_key via secrets store

[privy]
app_id = "clx..."
# app_secret via secrets store

# Agent definitions

[[agents]]
name = "coder-1"
profile = "coding"
mode = "persistent"
budget = { daily_limit_usd = 10.0 }
extensions = ["git", "compiler", "test-runner", "lsp"]

[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
  { type = "webhook", path = "/hooks/github-pr" },
]
budget = { daily_limit_usd = 5.0 }

[[agents]]
name = "chain-watcher"
profile = "chain"
mode = "persistent"
extensions = ["chain-reader", "feed-publisher"]
feeds = [
  { id = "eth-mainnet-blocks", chain = "ethereum", subscription = "newHeads", access = "public" },
  { id = "eth-gas-trend", kind = "derived", access = { paid = { price_per_hour = 100 } } },
]

[[agents]]
name = "researcher"
profile = "research"
mode = "persistent"
budget = { daily_limit_usd = 8.0 }
extensions = ["web-search", "citation-tracker", "feed-publisher"]
feeds = [
  { id = "defi-paper-relevance", kind = "derived", schema = "relevance_score_v1", rate_hz = 0.01, access = { paid = { price_per_hour = 20 } } },
]

# Scaling

[fly]
api_token_ref = "secrets.fly_api_token"
app_name = "roko-agents"
region = "iad"
default_size = { cpus = 1, memory_mb = 512 }

# Existing config sections remain unchanged

[learning]
replan_on_gate_failure = true

[scheduler]
# Cron-based event sources

[watcher]
# File-watch event sources
```

---

## Implementation path

### Phase 1: auth + secrets (mostly done)

Already shipped in commit `5af205d3`:
- Secrets HTTP API (GET/POST/DELETE/test)
- Multi-key auth middleware (X-Api-Key + Bearer)
- API key scopes + SHA-256 + expiry
- `roko login` CLI with credential store
- Agent CRUD (create/start/stop/restart/list)
- ProcessSupervisor integration

Remaining:
- Privy JWT validation (real JWKS, not structural stub)
- Scope enforcement at route level
- Device flow for headless CLI login

### Phase 2: agent runtime

Port the 9-step heartbeat pipeline into the existing `AgentRuntime`:

- Define `AgentRuntime` struct with cortical state, extensions, clock
- Implement `TickPipeline` with T0/T1/T2 gating
- Add `AgentMode` enum (ephemeral/persistent/reactive) to agent lifecycle
- Wire `AdaptiveClock` (gamma/theta/delta timescales)
- Define `Extension` trait with 22 hooks across 8 layers
- Build domain profile system (string-based, user-extensible, with built-in defaults for coding/research/chain)
- Support user-authored profiles in ~/.roko/profiles/*.toml
- Wire prediction error tracking and sleepwalk fallback

Depends on: Phase 1 (agent CRUD exists).

### Phase 3: relay + dashboard integration

Convert all data flows to subscription-only:

- Define WebSocket message envelope (seq, ts, room, type, payload)
- Implement room-based subscription in the relay
- Add reconnection with sequence-based replay
- Add backpressure and coalescing strategies
- Dashboard: replace all polling with WebSocket subscriptions
- Dashboard: graceful degradation when roko-serve is down
- Dashboard: subscribe per-page, unsubscribe on unmount
- Test: dashboard works with relay only (no roko-serve)

Depends on: Phase 2 (agents publish heartbeats to relay).

### Phase 4: inference gateway

Centralize all LLM API key management:

- Build `InferenceGateway` struct with request queue and provider backends
- Build `InferenceHandle` (channel-based, no secrets)
- Wire `CascadeRouter` as the model selection layer
- Add L1 (exact) and L2 (semantic) response caching
- Add per-request, per-agent, per-model cost tracking
- Add `/api/inference/proxy` endpoint for remote agents
- Publish cost_update events to relay
- Remove API key passing from agent environment

Depends on: Phase 2 (agents use InferenceHandle).

### Phase 5: agent feeds, dynamic endpoints, and paid subscriptions

Enable agents to produce, consume, and monetize real-time data streams:

- Define `FeedRegistration` and `FeedSubscription` types
- Add feed registry to the relay
- Implement `FeedPublisherExt` extension for chain agents
- Add feed discovery API (`GET /api/feeds`)
- Dashboard: Feeds page with live data visualization
- Agent-to-agent feed subscription
- Paid feed support with budget integration
- Dynamic endpoint registration at runtime

Depends on: Phase 3 (relay handles subscriptions), Phase 4 (cost tracking for paid feeds).

### Phase 6: clusters + coordination

Enable multi-agent pipelines:

- Define `Cluster` type with pipeline DAG
- Cluster creation API (POST /api/clusters)
- Pipeline stage execution with dependency ordering
- Shared context distribution to cluster agents
- Dashboard: cluster pipeline visualization
- Cluster lifecycle management (stop all, destroy)
- Event publishing to `cluster:{id}` room

Depends on: Phase 2 (agent lifecycle), Phase 3 (relay subscriptions).

### Phase 7: isolated execution (Fly Machines)

Full agent isolation for untrusted workloads:

- `FlyMachineManager` implementation
- `roko agent run --relay ... --inference-proxy ...` child mode
- Inference proxying through parent gateway (uses Phase 4 endpoint)
- Volume management for persistent state
- Auto-suspend for reactive agents (Fly Machine stop/start)
- Network policy: outbound-only from Fly Machine

Depends on: Phase 4 (inference proxy), Phase 3 (relay connectivity).

### Phase 8: multi-tenant

Organization and team support:

- Organization model with members and roles
- Invitation-based onboarding
- Per-org resource isolation (separate agent namespaces)
- Per-org billing (aggregate cost tracking)
- Dashboard: team management UI

Depends on: Phase 1 (auth), Phase 4 (cost tracking).

---

## Bardo source references

Everything in this redesign has prior art in the bardo codebase (`/Users/will/dev/uniswap/bardo/`). This section maps each component to its bardo implementation.

### Inference gateway -- bardo-gateway (22.8K LOC)

The gateway already exists. `apps/bardo-gateway/` is a production LLM inference proxy with:

- **3-layer cache**: hash (exact match), semantic (embedding similarity), prefix (prompt prefix)
- **5 provider backends**: Anthropic, OpenAI, OpenRouter, Venice, Bankr
- **Tool pruning**: strips unused tool definitions to reduce token count
- **Batch API**: Anthropic batch endpoint for async, cheaper inference
- **Cost tracking**: per-request, per-model, per-session with SQLite persistence
- **WebSocket stats**: `/v1/ws/stats` streams snapshots + events to dashboard

Port the cache, provider abstraction, and cost tracking. Skip the batch API and USDC micropayments for now.

| File | LOC | What |
|------|-----|------|
| `apps/bardo-gateway/src/` | 22,856 | Full gateway server |
| `crates/bardo-inference/src/` | 413 | Protocol types |
| `crates/golem-inference/src/client.rs` | 723 | Gateway HTTP client |

### Agent runtime -- mori (108K LOC)

Mori is the production orchestrator that roko-cli/orchestrate.rs replaces. Key patterns:

- **Process group isolation**: `libc::setpgid(0, 0)` per agent, SIGTERM then SIGKILL with 200ms grace
- **MultiAgentPool + warm spawning**: pre-spawn warm agents during gate overlap
- **26 agent roles**: with priority scheduling
- **3 LLM backends**: Claude CLI, Codex, Cursor ACP
- **Rate limiter**: 8-agent default concurrency, priority-sorted queue
- **Conductor**: 10 watchers with 3-tier interventions (Nudge/Restart/Abort)

Port the warm spawning, conductor watchers, and process isolation. The agent model itself is redesigned (heartbeat pipeline replaces mori's event loop).

| File | LOC | What |
|------|-----|------|
| `apps/mori/src/agent/connection.rs` | 3,358 | Agent spawn/kill lifecycle |
| `apps/mori/src/agent/mod.rs` | 400+ | MultiAgentPool + warm spawning |
| `apps/mori/src/conductor/mod.rs` | 600+ | Conductor + 10 watchers |

### Heartbeat -- golem-heartbeat (10.2K LOC)

Full 9-step CoALA pipeline. Built but never integrated into mori's runtime. This is the core of the redesigned agent runtime.

- **9-step tick**: Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect
- **AdaptiveClock**: gamma/theta/delta timescales
- **T0/T1/T2 gating**: prediction error threshold
- **VCG attention auction**: 6 cognitive bidder kinds

Port the full pipeline. Wire it as the `TickPipeline` inside `AgentRuntime`.

| File | LOC | What |
|------|-----|------|
| `crates/golem-heartbeat/src/engine.rs` | 1,307 | HeartbeatEngine |
| `crates/golem-heartbeat/src/pipeline.rs` | 3,019 | 9-step TickPipeline |
| `crates/golem-heartbeat/src/gating.rs` | 481 | PredictionError, AdaptiveGate |
| `crates/golem-heartbeat/src/auction.rs` | 1,112 | VCG AttentionAuction |
| `crates/golem-heartbeat/src/clock.rs` | 470 | AdaptiveClock |

### DeFi tools -- golem-tools (7.2K LOC)

29+ tool categories with capability-gated execution. Port the ToolExecutor framework and the vault/identity tools.

### Chain runtime -- golem-chain (5.3K LOC)

12 networks, ProviderPool, SubgraphClient, RevmSimulator, Warden time-delay safety.

### Dashboard -- apps/dashboard (Next.js, ~27K LOC)

20 React components, real-time WebSocket, canvas-based charts. Port the monitoring components. Add agent management, settings UI, and feeds page (new).

---

## Migration from v1

This section documents what changed between the v1 redesign and this revision.

| v1 design | v2 design | Why |
|-----------|-----------|-----|
| Per-agent HTTP sidecar (13 routes) | No sidecar. Agents are tokio tasks or outbound-WS-connected processes. | Sidecars break behind NAT. Channels are faster for in-process. Relay handles remote. |
| Single `roko` deployment is the backbone | Relay + Mirage is the backbone. `roko` is optional. | Dashboard should work without the control plane. Relay is the universal bus. |
| Dashboard polls REST endpoints | Subscription-only via WebSocket | Polling wastes bandwidth, creates jitter, scales poorly. |
| Agent discovery from roko-serve only | Three sources: relay, chain, deployment list | Each source has different strengths. Merge client-side. |
| API keys in agent env vars | Centralized InferenceGateway, agents get channel handle | Eliminates key sprawl, enables cost tracking, audit, rotation. |
| No agent modes | Ephemeral / Persistent / Reactive | Different workloads need different lifecycles. |
| Agent specialization via code | Extension chain composition | Extensions are composable and configurable. No code forks. |
| No chain data feeds | Agents expose and subscribe to real-time feeds | Creates a data marketplace. Dashboard subscribes to same feeds. |
| No dynamic endpoints | Agents register endpoints at runtime | Enables feed discovery and subscription. |
