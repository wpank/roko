# Agent runtime

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from v2 redesign doc.

---

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

### T0 reflex execution

T0 skips inference entirely. Instead it runs a rule engine over a local reflex store.

**Reflex store**: `.roko/learn/reflexes.jsonl`. Each line is a condition-action pair learned from previous T2 sessions. When a T2 decision produced a correct outcome (gate passed, no rollback) and the same observation pattern recurs, the decision gets promoted to a reflex rule.

```json
{"condition":{"tool":"bash","args_pattern":"cargo test.*","context":"gate_check"},"action":{"tool":"bash","args":"cargo test --workspace"},"confidence":0.97,"source_episode":"ep_a1b2c3","promoted_at":"2026-04-20T14:30:00Z"}
{"condition":{"message_type":"pr_review_request","file_ext":".rs"},"action":{"tool":"file_read","args":"{path}"},"confidence":0.91,"source_episode":"ep_d4e5f6","promoted_at":"2026-04-21T09:15:00Z"}
{"condition":{"tool":"git","args_pattern":"git status","context":"pre_commit"},"action":{"tool":"bash","args":"cargo +nightly fmt --all && cargo clippy --workspace --no-deps -- -D warnings"},"confidence":0.95,"source_episode":"ep_g7h8i9","promoted_at":"2026-04-22T11:00:00Z"}
```

**Execution flow**:

```
Observation arrives
       │
       ▼
Match against reflexes.jsonl (linear scan, conditions checked in order)
       │
  match found ──────► Execute action directly (no LLM)
       │                     │
  no match                   ▼
       │              Record outcome, update confidence
       ▼
  Escalate to T1
```

**Promotion criteria**: A T2 decision becomes a T0 reflex when:
- The same observation pattern triggers the same action 3+ times
- Every execution passed its gate (zero failures)
- Confidence > 0.90 (computed as success_count / total_count)

**Demotion**: If a reflex action fails a gate, its confidence is halved. Below 0.50, the rule is deleted and future matches escalate to T1.

### Adaptive clock algorithm

The clock adjusts tick frequency based on the agent's operating regime.

**Gamma interval** (fast perception tick):

```
gamma_interval = base_interval * regime_factor

base_interval = 500ms (configurable per agent)

Regime factors:
  Calm:     4.0x  →  2000ms between gamma ticks
  Normal:   1.0x  →   500ms between gamma ticks
  Volatile: 0.5x  →   250ms between gamma ticks
  Crisis:   0.25x →   125ms between gamma ticks
```

**Theta interval** (reflective planning tick):

```
theta_interval = N * gamma_interval

N varies by regime:
  Calm:     N = 8   →  16000ms (16s) between theta ticks
  Normal:   N = 5   →   2500ms (2.5s) between theta ticks
  Volatile: N = 3   →    750ms between theta ticks
  Crisis:   N = 2   →    250ms between theta ticks
```

**Delta interval** (deep consolidation tick):

Triggers on whichever comes first:
- `idle_timeout`: 60s of no observation activity (no new messages, no tool results)
- `episode_threshold`: 20 episodes accumulated since last delta tick

**Regime detection with hysteresis**:

```
                   ┌──────────────────────────────────────┐
                   │                                      │
                   ▼                                      │
              ┌─────────┐   PE > 0.40 for 3 ticks   ┌────┴────┐
     ┌───────►│  Calm    │─────────────────────────►│ Normal   │
     │        └─────────┘                            └────┬────┘
     │             ▲                                      │
     │   PE < 0.10 │  3 ticks                PE > 0.60   │  3 ticks
     │   3 ticks   │                          3 ticks    │
     │             │                                      ▼
     │        ┌────┴────┐                            ┌─────────┐
     │        │ Normal   │◄───────────────────────── │ Volatile │
     │        └─────────┘   PE < 0.30 for 3 ticks   └────┬────┘
     │                                                    │
     │                                       error_rate   │ > 0.5
     │                                       3 ticks      │
     │                                                    ▼
     │                                               ┌─────────┐
     └───────────────────────────────────────────────│ Crisis   │
               error_rate < 0.1 for 3 ticks          └─────────┘
```

The 3-tick hysteresis window prevents oscillation. A regime must persist for 3 consecutive gamma ticks before the clock adjusts. During the hysteresis window, the clock uses the previous regime's intervals.

### Cortical state persistence

Cortical state is serialized to `.roko/agents/{id}/cortical.json` on every theta tick.

```json
{
  "agent_id": "coder-1",
  "snapshot_at": "2026-04-24T14:32:10Z",
  "working_memory": [ ... ],
  "goals": [ ... ],
  "beliefs": { ... },
  "attention": { "focus": "implement auth middleware", "salience": 0.82 },
  "regime": "normal",
  "prediction_error_ema": 0.27,
  "episode_count": 142
}
```

**Restart behavior**:
- On agent startup, check for `.roko/agents/{id}/cortical.json`
- If the snapshot exists and is less than 1 hour old: load it and resume from the saved state
- If the snapshot is older than 1 hour: discard it, start with a fresh `CorticalState::default()`. Stale cortical state produces worse decisions than a cold start because goals, beliefs, and attention weights drift out of alignment with the actual environment.
- If no snapshot file exists: start fresh (first run)

### Extension chain

See [Extensions](03-extensions.md) for the full extension system, including the `Extension` trait, 8 layers, 22 hooks, domain profiles, and user-authored extensions.

### Domain profiles

> **Not a standalone primitive (PRD 23).** Domain is a field on Agent, not a separate primitive in the 12-primitive vocabulary. The `DomainProfile` string below maps to the `archetype.domain` field on `ArchetypeManifest`, which bundles domain, tool profiles, gate pipelines, model preferences, and behavioral constraints into a single agent template. This aligns with the existing design -- `DomainProfile` is already a string field on `AgentRuntime`, not an independent object with its own lifecycle.

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

## Acceptance criteria (added 2026-04-25)

> Backported from `tmp/architecture-plans/06-architecture-implementation.md` Phase A.

### AgentMode lifecycle

- [ ] Ephemeral agent stops after full task-gate-persist cycle completes (not on first response)
- [ ] Ephemeral timeout: 30 minutes of no completion → log warning and stop (configurable via `agent.ephemeral_timeout_secs`, default 1800)
- [ ] Persistent agent runs tick loop indefinitely until manually stopped
- [ ] Reactive agent sleeps between triggers (zero CPU). Webhook trigger wakes within 100ms. Cron trigger fires on schedule.
- [ ] `roko agent status --name x` shows `sleeping` for reactive agents between triggers

### T0/T1/T2 gating

- [ ] `decide_tier(0.10, 1000, 0.1)` returns T0
- [ ] `decide_tier(0.25, 1000, 0.5)` returns T1
- [ ] `decide_tier(0.50, 1000, 0.8)` returns T2
- [ ] `decide_tier(0.50, 0, 0.8)` returns Sleepwalk
- [ ] No hysteresis on tier decisions — evaluated fresh each tick (hysteresis is on clock regime only)

### T0 reflex store

- [ ] Reflex rule created after 3 identical T2 successes with zero gate failures
- [ ] T0 path matches rule and executes action without LLM call
- [ ] Gate failure halves reflex confidence
- [ ] Rule deleted when confidence < 0.50
- [ ] Mixed success/failure: confidence = success_count / total_count (running ratio)
- [ ] Max 200 rules, evict lowest confidence when full
- [ ] `.roko/learn/reflexes.jsonl` persists across restarts

### Adaptive clock

- [ ] Regime changes only after 3 consecutive qualifying ticks (hysteresis)
- [ ] Oscillating PE (e.g., 0.10, 0.20, 0.10) does NOT cause regime change — counter resets on non-qualifying tick
- [ ] Gamma interval: base × regime_factor (Calm=4.0, Normal=1.0, Volatile=0.5, Crisis=0.25)
- [ ] Delta tick fires on 60s idle OR 20 episodes accumulated (whichever first)
- [ ] `base_interval` configurable via `agent.clock_base_ms` in roko.toml

### Cortical state persistence

- [ ] Serialized to `.roko/agents/{id}/cortical.json` on every theta tick (not gamma)
- [ ] Snapshot < 1 hour old → loaded on restart
- [ ] Snapshot >= 1 hour old → discarded (stale beliefs hurt more than cold start)
- [ ] Working memory capped at 50 items (LRU eviction)
