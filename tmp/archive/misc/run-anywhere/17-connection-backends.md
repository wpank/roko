# Agent Connection Backends: Protocol Details for Claude, Cursor, and Codex

> **Audience**: Integration engineers, backend developers, protocol implementers
> **Scope**: Exact wire protocols, spawn commands, timeout parameters, and event mapping for all three LLM backends

---

## Three Distinct Process Models

| Backend | Binary | Process Model | Protocol | Session | Budget Control |
|---|---|---|---|---|---|
| **Claude** | `claude` CLI | Ephemeral per-turn subprocess | `stream-json` on stdout | `--resume <session-id>` | `--max-budget $1.50` |
| **Cursor** | `agent` CLI | Session-persistent ACP server | JSON-RPC 2.0 over stdio | `session_id` reuse | Not provided |
| **Codex** | `codex app-server` | Persistent JSON-RPC server | JSON-RPC 2.0 over stdio | `thread_id` reuse | Not provided |

---

## 1. Claude Connection (Ephemeral Subprocess)

### Spawn Command (Exact Flags)

```bash
claude \
  --bare \                              # Skip 30K-token default system prompt (92% reduction)
  --print \                             # Non-interactive mode
  --verbose \                           # Include tool call details
  --output-format stream-json \         # One JSON object per line on stdout
  --model claude-opus-4-6 \             # Model slug
  --effort max \                        # low | medium | high | max
  --append-system-prompt "..." \        # Role-specific system prompt (Layer 1 cache)
  --fallback-model claude-haiku-4-5 \   # Automatic fallback on primary failure
  --settings '{"hooks":{...}}' \        # Safety hooks (block dangerous git commands)
  --tools Read,Glob,Grep,Edit,Write,Bash,WebFetch,WebSearch \ # Tool allowlist
  --dangerously-skip-permissions \      # Skip interactive approval prompts
  --mcp-config /path/to/.mori/mcp-config.json \ # MCP server configuration
  --strict-mcp-config \                 # Enforce MCP schema validation
  --resume abc123                       # Session ID from prior turn
```

### stream-json Protocol (Exact Format)

```jsonl
{"type":"system","session_id":"abc123","model":"claude-opus-4-6"}
{"type":"assistant","message":{"content":[{"type":"text","text":"I'll read the file..."},{"type":"tool_use","id":"tu_01","name":"Read","input":{"file_path":"src/lib.rs"}}],"usage":{"input_tokens":5000,"output_tokens":1200}}}
{"type":"tool","content":"pub mod core;\npub mod agent;\n..."}
{"type":"result","subtype":"success","session_id":"abc123","is_error":false,"num_turns":3,"total_cost_usd":0.42,"usage":{"input_tokens":15000,"output_tokens":3600}}
```

### Budget Per Role

| Role | Base | Opus (2×) | Haiku (0.6×) |
|---|---|---|---|
| Implementer | $1.50 | $3.00 | $0.90 |
| AutoFixer | $0.75 | $1.50 | $0.45 |
| Strategist, Researcher | $0.75 | $1.50 | $0.45 |
| Conductor | $0.50 | $1.00 | $0.35 |
| Auditor, QuickReviewer | $0.50 | $1.00 | $0.35 |
| Scribe, Critic | $0.40 | $0.80 | $0.35 |
| All others | $0.50 | $1.00 | $0.35 |

Override globally via `MORI_CLAUDE_MAX_BUDGET_USD`.

### Safety Hooks (Block Dangerous Git)

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [
        {"type":"command","if":"Bash(git checkout *)","command":"echo 'BLOCKED' >&2 && exit 2"},
        {"type":"command","if":"Bash(git switch *)","command":"echo 'BLOCKED' >&2 && exit 2"},
        {"type":"command","if":"Bash(git branch -m *)","command":"echo 'BLOCKED' >&2 && exit 2"},
        {"type":"command","if":"Bash(git push *)","command":"echo 'BLOCKED' >&2 && exit 2"}
      ]
    }]
  }
}
```

### MCP Config Discovery (Search Order)

```
/worktree/.mori/mcp-config.local.json  (checked first)
/worktree/.mori/mcp-config.json
/root/.mori/mcp-config.local.json
/root/.mori/mcp-config.json
... up to filesystem root
Override: MORI_MCP_CONFIG environment variable
```

### Turn Lifecycle

```
Turn N:
  1. Spawn claude process with flags + stdin pipe
  2. Write prompt to stdin, close (EOF)
  3. Read stream-json events from stdout line-by-line
  4. Parse events → AgentEvent (MessageDelta, ToolCall, TokenUsage, etc.)
  5. On "result" event: extract session_id, cost, usage
  6. Process exits

Turn N+1:
  Same process flow, but with --resume <session_id from Turn N>
  Claude CLI internally rehydrates conversation history
```

---

## 2. Cursor ACP Connection (Session-Persistent)

### Spawn Command

```bash
agent \
  --force \                       # Force start even if another instance exists
  --approve-mcps \                # Auto-approve MCP server connections
  --workspace /path/to/dir \      # Working directory
  --output-format json \          # JSON-RPC output
  --model composer-2 \            # Model slug
  acp                             # Start in ACP mode
```

### JSON-RPC Handshake (Exact Sequence)

```json
// Phase 1: Initialize
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientInfo":{"name":"roko","version":"0.1.0"},"clientCapabilities":{}}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"serverInfo":{"name":"cursor-agent"}}}

// Phase 2: Create Session
→ {"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/path/to/worktree","mode":"agent","mcpServers":[]}}
← {"jsonrpc":"2.0","id":2,"result":{"sessionId":"sess-001"}}

// Phase 3: Send Prompt
→ {"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"sess-001","prompt":[{"type":"text","text":"Implement merge sort in Rust"}]}}
← (notifications stream) {"jsonrpc":"2.0","method":"session/update","params":{"kind":"agent_message_chunk","content":"I'll implement..."}}
← (notifications stream) {"jsonrpc":"2.0","method":"session/update","params":{"kind":"tool_call","toolName":"Edit","status":"started"}}
← (notifications stream) {"jsonrpc":"2.0","method":"session/update","params":{"kind":"tool_call_update","completed":true}}
← {"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}
```

**Critical notes**:
- `protocolVersion` must be integer `1`, not date string
- `mcpServers` must be `[]` even if empty (not omitted)
- `mode` must be `"agent"` (not `"plan"` or `"ask"`)
- Global startup lock (`AsyncMutex`) prevents race conditions in concurrent spawns

### Timeouts

| Operation | Timeout |
|---|---|
| Initialize | 90 seconds |
| session/new | 60 seconds |
| Kill (stdin close) | 1.2 seconds |
| Kill (SIGTERM) | 800ms |

---

## 3. Codex App-Server Connection (Persistent JSON-RPC)

### Spawn Command

```bash
codex app-server \
  -c model_reasoning_effort="high" \
  --disable plugins \
  --disable tool_suggest \
  -c 'model="o3"' \
  -c 'service_tier="fast"' \
  -c 'mcp_servers.mori.command="/path/to/roko-mcp"' \
  -c 'mcp_servers.mori.args=["--workspace","/path"]' \
  -c 'mcp_servers.mori.required=true' \
  -c 'mcp_servers.mori.startup_timeout_sec=60'
```

### JSON-RPC Requests (Exact Format)

```json
// Initialize
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"roko","version":"0.1.0"}}}

// Create thread
→ {"jsonrpc":"2.0","id":2,"method":"thread/start","params":{}}
← {"jsonrpc":"2.0","id":2,"result":{"threadId":"t-001"}}

// Start turn
→ {"jsonrpc":"2.0","id":3,"method":"turn/start","params":{"input":[{"type":"text","text":"Fix the failing test"}],"threadId":"t-001"}}

// Approval request (server → client, MUST preserve id type)
← {"jsonrpc":"2.0","id":"approval-42","method":"item/commandExecution/requestApproval","params":{"command":"cargo test"}}
→ {"jsonrpc":"2.0","id":"approval-42","result":{"decision":"accept"}}
```

### Notification → AgentEvent Mapping

| Codex Notification | AgentEvent |
|---|---|
| `item/agentMessage/delta` | `MessageDelta` |
| `turn/completed` | `TurnCompleted` |
| `thread/updated` (status=idle) | `TurnCompleted` |
| `turn/diff/updated` | `DiffUpdated` |
| `item/commandExecution/requestApproval` | `ApprovalRequested` |
| `thread/tokenUsage/updated` | `TokenUsage` |
| `item/commandExecution/outputDelta` | `CommandOutput` |

### Isolated CODEX_HOME

Each agent instance gets its own `CODEX_HOME`:
```
.mori/runtime/codex-home/<role>/<instance>/
```
Prevents state collisions between concurrent Codex agents. Recreated if deleted.

### Stderr Suppression (25+ Categories)

Codex emits numerous benign warnings that must be filtered: `apply_patch_verification`, `state-db-migration`, `shell-snapshot-enoent`, `thread-shutdown-timeout`, `process-group-kill`, `unknown-model-metadata`, `sqlx-slow-statement`, `source-snippet`, and ~17 more.

### Thread Creation Retry

Up to 3 attempts with `500ms × attempt` backoff. Reasoning effort levels: `low`/`medium`/`high`/`xhigh`.

---

## 4. Shared Infrastructure

### Process Lifecycle (All Backends)

```
Spawn:
  1. setpgid(0, 0)  — isolate in own process group
  2. Configure stdio (piped stdin/stdout/stderr)
  3. Set environment variables
  4. Register PID in .mori/runtime/agent-pids.json

Kill sequence (escalating):
  1. Close stdin (EOF) → wait 1.2s
  2. SIGTERM to process group (-pgid) → wait 800ms
  3. SIGKILL to process group → wait 800ms
  4. child.kill() fallback
  5. Wait 800ms for confirmation

Descendant collection:
  - pgrep -P <pid> before kill (catch processes that escape group)

Orphan reaper:
  - Periodic check for processes with parent PID 1 (reparented to init)
  - Kill and prune from PID registry

Nuclear shutdown:
  - kill_all_mori_descendants() on app exit
```

### Shared Environment Variables

```bash
CARGO_INCREMENTAL=0           # Disable incremental (helps sccache)
RUSTC_WRAPPER=sccache         # Compiler caching
SCCACHE_BASEDIRS=/path/root   # Normalize paths across worktrees
CARGO_BUILD_JOBS=2            # Limit parallelism (20 agents → 40 rustc max)
CARGO_TARGET_DIR=/path/to/worktree/.mori/runtime/target
FASTEMBED_CACHE_DIR=/shared/path
ORT_CACHE_DIR=/shared/path
```

### Backend Routing (Exact Logic)

```rust
pub fn from_model(slug: &str) -> AgentBackend {
    if slug.starts_with("claude-") {
        AgentBackend::Claude
    } else if is_cursor_model_slug(slug) {
        AgentBackend::Cursor
    } else {
        AgentBackend::Codex  // default
    }
}

fn is_cursor_model_slug(slug: &str) -> bool {
    slug.starts_with("composer-") || slug.starts_with("cursor-")
        || slug == "auto"
        || slug.starts_with("sonnet-") || slug.starts_with("opus-")
        || slug.starts_with("haiku-") || slug.starts_with("gemini-")
        || slug.starts_with("kimi-") || slug == "gpt-5.2"
        || slug.ends_with("-high") || slug.ends_with("-xhigh-fast")
}
```

### Unified AgentEvent Stream

All three backends emit the same event types:

```rust
pub enum AgentEvent {
    MessageDelta { role: AgentRole, instance: String, content: String },
    TurnCompleted { role: AgentRole, instance: String, thread_id: Option<String> },
    DiffUpdated { role: AgentRole, instance: String, diff: String },
    ApprovalRequested { role: AgentRole, instance: String, command: String, approval_id: Value },
    TokenUsage { role: AgentRole, instance: String, input_tokens: u64, output_tokens: u64, cost_usd: Option<f64> },
    ToolCall { role: AgentRole, instance: String, name: String },
    CommandOutput { role: AgentRole, instance: String, content: String },
    Error { role: AgentRole, instance: String, error: String },
    Exited { role: AgentRole, instance: String, exit_code: Option<i32> },
}
```

### Stream Batching Thresholds

| Event Type | Buffer Size | Flush Trigger |
|---|---|---|
| MessageDelta | 256 bytes | Newline OR buffer full |
| CommandOutput | 512 bytes | Newline OR buffer full |
| All others | N/A | Immediate (also flushes pending batches) |

---

## 5. Agent Pools (Sequential and Parallel)

### AgentPool (Sequential Mode)

One connection per role. Simple lifecycle:

```rust
pub struct AgentPool {
    connections: HashMap<AgentRole, AgentConnection>,
    working_dir: PathBuf,
    event_tx: mpsc::UnboundedSender<AgentEvent>,
    fast_mode: bool,
    fallback_model: Option<String>,
}
```

Operations: `spawn(role, effort, model)` → `turn_start(role, prompt)` → stream events → `kill(role)`.

If primary model fails, automatically retries with `fallback_model`.

### MultiAgentPool (Parallel Mode)

Multiple instances per role. Instance IDs: `(role, identifier)` e.g., `"implementer:plan-42-group-A"`.

```rust
pub struct MultiAgentPool {
    connections: HashMap<AgentInstanceId, AgentConnection>,
    warm_pool: HashMap<(AgentRole, String), AgentConnection>,
    default_working_dir: PathBuf,
    fast_mode: bool,
}
```

**Instance lifecycle**:

```
Spawned → Active → Running → Done/Failed/Cancelled → Terminal
                                     ↓
                              recycle_terminal_to_warm (optional)
```

**Concurrency limit**: Default 4 per role. Configurable via `set_concurrency_limit(role, limit)`.

**Warm pool operations**:
- `pre_spawn_warm(role, count, agent_fn)` — Pre-initialize agents without starting work
- `promote_warm(role)` — Move from warm pool to active (instant, no cold start)
- `evict_warm(role)` — Kill unused warm agents
- `promote_warm_if_capacity(role)` — Only promote if under concurrency limit

**Kill operations**:
- `kill_all(deadline)` — Sweep active first, then warm. `KillReport` tracks killed + aborted.
- `kill_plan_agents(plan_id)` — Kill only agents for a specific plan
- `kill_role(role)` — Kill all instances of a role
- `reap_terminal_active()` — Clean up completed instances

### AgentInstanceId

```rust
pub struct AgentInstanceId {
    pub role: AgentRole,
    pub instance: String,  // e.g., "plan-42-group-A"
}
```

### Express Mode (Streamlined Pipeline)

For simple/trivial plans:

```
Implementer → Compile Gate → Test Gate →
  {pass: Merge, fail: AutoFixer → Compile → Test → ...}
```

No Strategist, no reviews, no Critic. Max auto-fix attempts: 3 (configurable).

---

## 6. Failure Classification

When a plan fails, the failure is classified for appropriate recovery:

```rust
pub enum FailureKind {
    AutoFixExhausted,        // Express: auto-fix attempts exhausted
    AllTasksFailed,          // Every task failed
    TaskRetriesExhausted,   // Single task exceeded retry budget
    SetupFailed,            // Worktree/file setup issues
    MaxIterations,          // Too many revision cycles
    SpawnFailures,          // Consecutive agent spawn failures
    Deadlock,               // Merge queue deadlock
    WorktreeMissing,        // Git worktree disappeared
    VacuousImplementation,  // Agent wrote no code after retries
    Other(String),          // Transient/unclassified
}
```

**Recoverable**: SpawnFailures, Deadlock, WorktreeMissing, Other → automatic retry.
**Terminal**: AutoFixExhausted, AllTasksFailed, VacuousImplementation → requires human intervention or re-planning.

---

## 7. The Seven Usage Modes

The connection layer supports seven deployment patterns:

| Mode | Interface | Detection | Use Case |
|---|---|---|---|
| **Interactive REPL** | Terminal | `isatty(stdin)` | Developer conversation |
| **One-Shot** | CLI argument | Prompt arg provided | Single task, exit code |
| **Pipe** | stdin | `!isatty(stdin)`, no arg | Process piped input |
| **Orchestrated** | Programmatic | Library API call | Plan DAG execution |
| **GitHub Bot** | Webhooks | Server mode | Auto-triage, auto-review |
| **CI Agent** | One-shot in Actions | CLI in workflow | Fix tests, review PRs |
| **HTTP Service** | REST + SSE | `serve` subcommand | Agent-as-a-Service |

Each mode uses the same underlying `AgentPool` or `MultiAgentPool` — the connection and event streaming are identical regardless of how the agent is invoked.

---

## 8. Agent Pools: Managing Multiple Concurrent Agents

### Three Pool Strategies

Agent execution ranges from simple single-task debugging to massively parallel plan execution with dozens of concurrent agents. Three pool strategies serve these different needs:

**Sequential Pool** (`AgentPool`): One agent at a time per role. The simplest model — spawn an agent, send a prompt, wait for completion, send the next prompt. No concurrency management, no warm pool, no scheduling. Used for debugging, single-task execution, and interactive REPL mode.

**Parallel Pool** (`MultiAgentPool`): N agents simultaneously, each in its own git worktree. The DAG scheduler determines which tasks are ready (all dependencies satisfied), assigns them to available agents, and manages their lifecycle. Used for plan execution where independent tasks can run concurrently.

**Warm Pool** (extension of `MultiAgentPool`): Pre-spawned agents waiting in idle state. When a task becomes ready, the pool promotes an idle agent to active status — **0ms startup latency** versus 5-15 seconds for cold-starting a new agent process. If a task's gates fail before the warm agent is needed (e.g., the reviewer was pre-spawned but the implementer's code failed compilation), the warm agent is evicted with zero token waste — it never received a prompt, so no inference cost was incurred.

### The MultiAgentPool State Machine

Each agent instance follows a defined lifecycle:

```
                    ┌──────────────────────┐
                    │                      │
                    ▼                      │
               WarmIdle ──────────────► Dropped
                    │                      ▲
                    │ promote()             │
                    ▼                      │
                 Active                    │
                    │                      │
                    │ turn_start()         │
                    ▼                      │
                 Running                   │
                    │                      │
                    ├── success ──► Terminal ──► Dropped
                    ├── failure ──► Terminal ──► WarmIdle (recycle)
                    └── cancel  ──► Terminal ──► Dropped
```

**WarmIdle**: Agent process is spawned and initialized (handshake complete, MCP tools discovered, session created) but has not received any prompt. Token cost: zero. CPU cost: minimal (process waiting on stdin). Memory cost: ~30-80MB per warm agent (model-dependent).

**Active**: Agent has been promoted from WarmIdle and assigned a task. It has received its context pack and system prompt but has not yet started inference.

**Running**: Agent is actively processing — reading files, writing code, calling tools, producing output. Events stream through the `AgentEvent` channel.

**Terminal**: Agent's turn is complete (success, failure, or cancellation). The instance can be recycled back to WarmIdle (if the process supports session reuse) or dropped (killed).

**Dropped**: Agent process has been killed and its resources released. The worktree may be retained for inspection.

### Concurrency Limits and Capacity Checks

```rust
impl MultiAgentPool {
    pub fn set_concurrency_limit(&mut self, role: AgentRole, limit: usize) {
        // Default: 4 per role
        // Implementer: often higher (8-12) for parallel plan execution
        // Conductor: always 1 (singleton orchestrator)
        self.limits.insert(role, limit);
    }

    pub fn at_capacity(&self, role: AgentRole) -> bool {
        let active = self.connections.iter()
            .filter(|(id, _)| id.role == role)
            .count();
        active >= self.limits.get(&role).copied().unwrap_or(4)
    }

    pub fn promote_warm_if_capacity(&mut self, role: AgentRole) -> Option<AgentInstanceId> {
        if self.at_capacity(role) {
            return None;  // Would exceed limit
        }
        self.promote_warm(role)  // Move from warm to active
    }
}
```

**Why limits matter**: Without concurrency limits, a burst of ready tasks could spawn 50 agent processes simultaneously — exhausting system memory, CPU, and API rate limits. The limits ensure graceful degradation: excess tasks queue rather than crash.

### Kill Operations

Three granularities of kill, each with different use cases:

**`kill_all(deadline)`**: Sweeps all agents — active first (they are doing work that should be stopped), then warm (they are consuming resources with no assigned work). Returns a `KillReport`:

```rust
pub struct KillReport {
    pub killed_active: usize,    // Active agents that were killed
    pub killed_warm: usize,      // Warm agents that were evicted
    pub aborted: usize,          // Agents that did not exit cleanly within deadline
    pub elapsed: Duration,       // Total time for kill sweep
}
```

**`kill_plan_agents(plan_id)`**: Kills only agents associated with a specific plan. Used when a plan fails and its agents should be cleaned up without affecting other running plans.

**`kill_role(role)`**: Kills all instances of a specific role. Used when a role is misbehaving (e.g., all reviewers are stuck in a loop) and needs a fresh start.

**Kill sequence** (per agent, escalating):
1. Close stdin (EOF signal) — wait 1.2s for graceful exit
2. SIGTERM to process group — wait 800ms
3. SIGKILL to process group — wait 800ms
4. `child.kill()` fallback — wait 800ms for confirmation
5. Descendant collection via `pgrep -P <pid>` — catch escaped processes

### Warm Pool Economics

The warm pool trades memory for latency. The economic calculation:

| Factor | Cold Start | Warm Promotion |
|---|---|---|
| Startup latency | 5-15s (process spawn + handshake + MCP discovery) | 0ms (already initialized) |
| Memory cost | 0 (process doesn't exist yet) | 30-80MB per warm agent |
| Token cost | 0 | 0 (no prompt until promotion) |
| Wasted cost if task cancelled | 0 | 0 (no inference was run) |

**Break-even**: If a plan has 10 sequential task groups and each group triggers a 10s cold start, the total startup overhead is 100s. Pre-warming 4 agents costs ~200MB but eliminates ~40s of waiting (4 parallel promotions instead of 4 sequential cold starts). For plans running on expensive Opus models ($15/M tokens), the time saved often exceeds the memory cost.

---

## 9. Failure Classification and Recovery

### How Different Failure Types Are Handled

Not all failures are equal. A rate limit that resolves in 30 seconds requires a different response than an authentication error that will never resolve on retry. Roko classifies every failure into one of four categories, each with a distinct recovery strategy.

### Transient Failures (Automatic Retry)

Failures that are likely to resolve on their own within seconds to minutes:

| Failure | Detection | Recovery |
|---|---|---|
| Rate limit (429) | HTTP status code | Exponential backoff: 1s, 2s, 4s + jitter. Max 3 retries. |
| Network timeout | No response within 90s | Retry with same parameters. Max 3 retries. |
| Connection reset | TCP RST or EOF | Respawn connection, retry. Max 3 retries. |
| Server error (500/502/503) | HTTP status code | Exponential backoff: 2s, 4s, 8s. Max 3 retries. |

**Jitter**: Random delay added to backoff to prevent thundering herd. Formula: `base_delay * 2^attempt + random(0, base_delay)`. Without jitter, all agents hitting a rate limit retry simultaneously, causing another rate limit.

**Budget accounting**: Retried requests count toward the agent's token budget. If the budget is exhausted during retries, the failure escalates to a budget failure.

### Persistent Failures (Circuit Breaker + Failover)

Failures that indicate a systemic problem with the provider:

| Failure | Detection | Recovery |
|---|---|---|
| Authentication error (401/403) | HTTP status code | Circuit breaker trips immediately. No retry — the credential is invalid. |
| Invalid model (404 on model endpoint) | API response | Failover to alternative model. Log deprecation warning. |
| API breaking change | Unexpected response schema | Circuit breaker trips. Alert operator. |
| Provider outage | 5+ consecutive 500s within 60s | Circuit breaker trips. Failover to secondary provider. |

### The 3-State Circuit Breaker

The circuit breaker prevents wasting tokens and time on a provider that is persistently failing:

```
                    success
           ┌────────────────────┐
           │                    │
           ▼                    │
        Closed ──── failure ──► Open
        (normal)    threshold   (all requests fail fast)
           ▲                    │
           │                    │ 30s cooldown
           │                    ▼
           └───── probe ────── HalfOpen
                  succeeds     (send 1 probe request)
                               │
                               │ probe fails
                               ▼
                              Open
                              (reset 30s timer)
```

**Closed** (normal operation): Requests flow through normally. A failure counter tracks consecutive failures. When the counter reaches the threshold (default: 5 consecutive failures), the breaker trips to Open.

**Open** (fail-fast): All requests immediately return a `CircuitBreakerOpen` error without contacting the provider. This lasts for 30 seconds (configurable). No tokens wasted, no latency waiting for timeouts.

**HalfOpen** (probing): After the cooldown, the breaker allows exactly one request through as a probe. If the probe succeeds, the breaker returns to Closed (provider has recovered). If the probe fails, the breaker returns to Open (provider is still down) and resets the 30s timer.

### Provider Failover Chain

Each model intent type has a configured failover chain:

```
Primary: Claude Opus 4.6 (via Claude CLI)
  │ circuit breaker trips
  ▼
Secondary: Claude Sonnet 4.5 (via Claude CLI, --fallback-model)
  │ circuit breaker trips
  ▼
Tertiary: Codex o3 (via Codex app-server)
```

**Intent-type restrictions**: Risk-critical intents (DeFi safety checks, PolicyCage verification) are restricted to trusted providers. They do not fall back to untrusted or lower-capability models. A safety check that fails because the primary provider is down escalates to the operator rather than running on a less capable model.

**Model capability matching**: Failover targets are pre-validated for capability compatibility. A task requiring tool use cannot fail over to a model that does not support tool use. A task requiring 128K context cannot fail over to a model with 32K context.

### Semantic Failures (Escalate, Do Not Retry)

Failures where the model understands the request but refuses or cannot complete it:

| Failure | Detection | Recovery |
|---|---|---|
| Safety refusal | Model output contains refusal pattern | Log exact refusal. Escalate to operator. Do NOT retry with same prompt. |
| Task beyond capability | Model explicitly states inability | Escalate to operator for re-planning. |
| Harmful content block | Provider-level content filter | Log filter trigger. Review task for policy compliance. |

**Why no retry**: Retrying a safety refusal with the same prompt will produce the same refusal (or worse, the model may comply on retry with lower quality). Retrying with a modified prompt is prompt engineering — a human decision, not an automated one.

### Budget Failures (Graceful Degradation)

Failures where the cost of completing the task exceeds the allocated budget:

| Failure | Detection | Recovery |
|---|---|---|
| Token limit exceeded | `total_tokens > budget.max_tokens` | Compress context (drop low-priority sections), retry with smaller prompt. |
| Cost exceeded | `cost_usd > budget.max_cost` | Downgrade model tier (Opus → Sonnet → Haiku). If still over budget, abort. |
| Time exceeded | `elapsed > budget.max_time` | Warn via conductor. If >2x budget, abort. |

**Model tier downgrade**: The CascadeRouter maintains performance statistics per model per task type. When a budget failure triggers a downgrade, the router selects the next-cheapest model that has demonstrated acceptable quality for this task type. A task that has historically required Opus will not be downgraded to Haiku — it will be downgraded to Sonnet, and only if Sonnet's historical pass rate for this task type exceeds 70%.

**Abort with context**: When a task is aborted due to budget failure, the abort event includes the partial work completed, the budget consumed, and a recommendation for the re-planner (e.g., "Split this task into smaller subtasks" or "This task requires Opus — increase budget allocation").
