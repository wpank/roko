# Agent Runtime, Tool System, Extensions, and Safety Architecture

> **Audience**: Technical architects, developers building on roko, integration partners
> **Scope**: The execution layer — how agents actually run, dispatch tools, stay safe, and extend

---

## The Agent Runtime (bardo-runtime)

The runtime is a standalone crate with **zero roko dependencies** — pure process lifecycle management, event broadcasting, cancellation, and resource accounting. This isolation is deliberate: the runtime works for any supervised process system, not just LLM agents.

### ProcessSupervisor

Manages a pool of child processes (agent backends) with:
- **Spawn**: Allocate PID, configure stdio piped, start process, return handle
- **Shutdown**: Graceful SIGKILL → grace period (5s default) → force-kill
- **Reap**: Non-blocking check for processes that already exited (orphan prevention)
- **Shutdown-all**: Sequential termination of all tracked processes

Each process gets a unique `ProcessId` (monotonic atomic counter) and a `CancelToken` (hierarchical — cancelling a plan cancels all its agents).

**Research**: OTP supervision trees (Armstrong, 2003) — hierarchical fault tolerance. The ProcessSupervisor is a lightweight, single-level version of Erlang's supervision model.

### EventBus

A generic typed broadcast bus with bounded replay ring:

```rust
pub struct EventBus<E: Clone + Send + Sync + 'static> {
    tx: broadcast::Sender<Envelope<E>>,     // Live broadcast
    ring: Mutex<VecDeque<Envelope<E>>>,     // Replay buffer
    seq: AtomicU64,                         // Monotonic sequence number
}
```

- **Live subscribers** receive events in real-time via `tokio::sync::broadcast`
- **Late joiners** can replay missed events from the ring buffer
- **Sequence numbering** enables gap detection and ordered replay
- **Lock-free reads** after initialization (mutex only protects ring writes)

The EventBus is the backbone for all observability — every tool call, gate result, routing decision, and efficiency event flows through it.

### CancelToken (Hierarchical)

A lightweight alternative to `tokio_util::CancellationToken`:

```
Plan Root Token
  ├── Agent A Token (child)
  │     ├── Tool Call 1 Token (grandchild)
  │     └── Tool Call 2 Token (grandchild)
  └── Agent B Token (child)
```

Cancelling the root cancels everything. Cancelling a child doesn't affect siblings. Uses `AtomicBool` with `Release/Acquire` ordering — zero-overhead when not cancelled.

### ResourceAccount

Budget tracking with four dimensions:
- **Tokens**: input + output token count (u64)
- **Cost**: USD spent (f64)
- **Time**: wall-clock elapsed
- **Utilization**: percentage of budget consumed per dimension

Tier presets: `trivial()`, `simple()`, `standard()`, `complex()` — each with calibrated budgets.

---

## The Tool Dispatch Pipeline

### 16 Built-in Tools

| Tool | Category | Permission | Concurrent? | Description |
|---|---|---|---|---|
| `read_file` | Read | read | Parallel | Read UTF-8 file from worktree |
| `write_file` | Write | write | Parallel | Write content, create if absent |
| `edit_file` | Write | write | Serial | Replace exact string match in file |
| `multi_edit` | Write | write | Per-policy | Batch edits across multiple files |
| `apply_patch` | Write | write | Serial | Apply unified diff patch |
| `glob` | Read | read | Parallel | Find files matching glob pattern |
| `grep` | Read | read | Parallel | Search file contents (regex/literal) |
| `ls` | Read | read | Parallel | List directory contents |
| `bash` | Exec | exec | Serial | Execute shell command via `bash -c` |
| `run_tests` | Exec | exec | Serial | Run test suite (120s timeout) |
| `web_fetch` | Network | network | Parallel | HTTP GET, return body |
| `web_search` | Network | network | Parallel | Query search provider, return results |
| `notebook_edit` | Write | write | Serial | Edit Jupyter notebook cells |
| `task` | Special | dispatch | Serial | Launch sub-agent for focused work |
| `todo_write` | Meta | meta | Parallel | Manage per-turn todo list |
| `exit_plan_mode` | Meta | special | Serial | Submit plan for approval |

### The 7-Step Dispatch Pipeline

Every tool call passes through seven stages:

```
1. VALIDATE    → Check args against JSON schema
2. RESOLVE     → Look up ToolDef by canonical name
3. AUTHORIZE   → Check role permissions (read/write/exec/git/network)
4. SAFETY      → Pre-execution policy checks (path, bash, git, network, rate limit)
5. EXECUTE     → Race handler against timeout + cancellation token
6. TRUNCATE    → Limit result to max_result_bytes (16KB default, UTF-8 aware)
7. SCRUB       → Remove secrets from output (API keys, tokens, passwords)
```

**Batch dispatch** partitions calls by concurrency policy:
- **Parallel tools** (read_file, glob, grep, web_fetch): `futures::join_all()` — all run concurrently
- **Serial tools** (bash, edit_file, run_tests): sequential loop — preserves shell state, prevents write-write races

**Research**: Principle of least privilege (Saltzer & Schroeder, 1975). Each role gets exactly the tools it needs. Fewer tools = model wastes fewer tokens considering unavailable tools = higher accuracy.

### Dynamic Tool Registry (MCP Integration)

The `DynamicToolRegistry` composes static built-in tools with dynamically discovered MCP tools:

```rust
pub struct DynamicToolRegistry {
    base: Arc<dyn ToolRegistry>,           // StaticToolRegistry (16 built-ins)
    mcp_tools: HashMap<String, Vec<ToolDef>>,  // Per-server MCP tools
    all_tools: Vec<ToolDef>,               // Flattened, deduplicated view
}
```

MCP tools are discovered via the `tools/list` JSON-RPC method and converted to roko `ToolDef` structs via `mcp_to_tool_def()`. The registry deduplicates by name (configurable precedence: MCP wins or built-in wins).

### TypeScript Sidecar (Unix Domain Sockets)

For complex DeFi interactions, roko completely avoids implementing complex EVM protocol math natively in Rust. Instead, it employs a **TypeScript Sidecar** connected via ultra-low latency Unix Domain Sockets (~1-5ms IPC).
- **Why Node.js?**: Standard libraries like Uniswap SDK (for V3/V4 concentrated liquidity positions, router optimization, math, and calldata encoding) are almost exclusively written, maintained, and audited in TypeScript. Porting them faithfully to Rust would take months and inherently lag behind protocol updates.
- **IPC Protocol**: Roko sends structured JSON requests over the UDS connection. The TS Sidecar executes the audited SDK code, handles complex number representations, generates raw calldata or predictions, and returns standard ABI-encoded bytes back to the Rust runtime for signing and execution.

---

## The Safety Layer

Five independent policy families compose into a defense-in-depth stack:

### 1. Path Policy
Validates file paths stay within the worktree. Prevents `../../../etc/passwd` traversals via canonicalization. Used by all file-touching tools.

### 2. Bash Policy
Regex-based command allow/deny list for the `bash` tool:
- **Default deny**: `rm -rf /`, `sudo`, `dd if=/dev/`, fork bombs, `mkfs`
- **Configurable**: Additional patterns in `roko.toml`
- **Git protection**: Blocks `git push --force origin main`, `git checkout *`, `git branch -m *`

### 3. Network Policy
Blocks outbound requests to private IPs (127.0.0.1, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16). Prevents agent from accessing internal services.

### 4. Secret Scrubbing
Post-execution regex scrubbing of outputs:
- Patterns: `sk-ant-*`, `sk-proj-*`, `Bearer *`, `AKIA*` (AWS keys), etc.
- Applied to every `ToolResult::Ok` payload before the agent sees it

### 5. Rate Limiting
Per-tool, per-role rate limits:
- Default: 100 calls per 60-second window
- Prevents runaway tool invocation loops
- Tracked via sliding window: `HashMap<(role, tool), VecDeque<Instant>>`

### The Strict 3-Tier Capability Model (Compile-Time Safety)

At the Rust compiler level, the capability system explicitly distinguishes between read, write, and privileged tools:

1. **`ReadTool`**: Pure observation. Gathering price feeds, reading contracts, analyzing memory. Grants no ability to modify the state of the blockchain or the filesystem.
2. **`WriteTool`**: Standard mutation. Committing routine transactions, updating playbook records, adjusting minor settings.
3. **`PrivilegedTool`**: High-risk operations. Withdrawing funds, signing delegations, spawning replicants, transferring assets, or disabling safeties. These explicitly require human-in-the-loop owner signatures or hardcoded overrides.

```rust
pub struct Capability<T> {
    _marker: PhantomData<T>,
    // Cannot be: created outside safety crate (pub(crate) constructor)
    //            cloned (no Clone/Copy)
    //            used twice (consumed by value on use)
    //            forged (no Default)
}
```

Even if the LLM is fully compromised by prompt injection:
- It cannot create a `Capability<PrivilegedTool>` (Rust's type system prevents it).
- It cannot reuse an already-consumed capability (Rust's ownership and borrow checker guarantee single-use).
- It cannot call a `PrivilegedTool` without the explicitly matching capability context.

**Research**: Capability-based security (Dennis & Van Horn, 1966). This is the exact same pattern used by CloudABI, Capsicum, and WASI — applied directly to the agent tool dispatch layer.

### PolicyCage (On-Chain Safety Enforcement)

For DeFi agents, a Solidity smart contract enforces hard limits that **even a fully compromised LLM cannot bypass**:

```
PolicyCage enforces:
  ├── Approved assets whitelist (unapproved tokens → revert)
  ├── Approved protocols whitelist (unapproved contracts → revert)
  ├── Max position: 25% of portfolio (default 2500 bps)
  ├── Max concentration: 30% per protocol (default 3000 bps)
  ├── Min collateral ratio: 125% (default 12500 bps)
  ├── Max drawdown: 20% per day (default 2000 bps, range 500-5000)
  ├── Spending caps: $10K/tx, $50K/session, $100K/day (defaults)
  ├── Rebalance limits: max 24/day, min 1hr between
  ├── Strategy whitelist: only approved function selectors
  └── Sanctions oracle: blocks sanctioned addresses
```

**Constitution hash**: `keccak256(serialized_parameters)` stored on-chain, verified at agent boot. If parameters diverge from what the agent expects, it refuses to start.

**Modification timelocks**: Adding assets takes 1 hour. Changing drawdown limits takes 24 hours. Disabling sanctions oracle takes 48 hours. No instant changes to safety parameters.

**High Water Mark**: Drawdown calculated from signed HWM (owner signs via EIP-712, not self-reported by agent). Data source: Chainlink/Uniswap TWAP — agent cannot inflate its own performance.

### Three-Mode Custody

The safest wallet is the one the agent never holds. The custody architecture determines who holds the keys, what they can sign, and what happens to funds when the agent is deleted. Three modes provide different trust models:

1. **Delegation (Recommended)**: Funds never leave the owner's MetaMask Smart Account. The agent holds a disposable session key and a signed ERC-7710/7715 delegation with on-chain caveat enforcers. Every transaction executes from the owner's address. If the session key leaks, damage is bounded purely by math. Owner revokes with one MetaMask click — no agent cooperation needed.
2. **Embedded (Privy)**: Legacy/Custodial mode. Funds transfer to a Privy server wallet running in AWS Nitro Enclaves. Simpler setup, but owner surrenders direct custody and must trust Privy's TEE integrity. Requires an explicit sweep at death.
3. **LocalKey+Delegation (Dev)**: Locally generated keypair (secp256k1) bounded by an on-chain delegation. The key is insecure in the traditional sense, but the on-chain delegation isolates the blast radius entirely. Use for hackathons and local validation.

#### The 7 Custom Caveat Enforcers

When utilizing the **Delegation** mode, seven specific Solidity caveat enforcers definitively bound the agent's absolute power. An attacker stealing the Session Key cannot break these on-chain guarantees:
1. **`GolemPhaseEnforcer`**: Restricts actions by behavioral phase via the VitalityOracle. A Conservation-phase agent algorithmically cannot open new positions, even if it tries.
2. **`MortalityTimeWindow`**: Auto-expires delegations block-timestamp > end time, forcing biological-clock-like termination constraints onto the session key.
3. **`DreamMode`**: Imposes structural/network atonia. Blocks all write actions while the agent is designated "dreaming," completely severing action paths when offline reflection activates.
4. **`VaultNAV`**: Enforces strict percentage-based single-trade caps (e.g., cannot move more than 5% of Net Asset Value in a single signature).
5. **`ReplicantBudgetEnforcer`**: Limits the total cumulative USD spend of spawned replicants (sub-agent delegation attenuation), so they can never exceed their parent's specific budget allocations.
6. **`MaxSlippage`**: Bounds the acceptable slippage natively by reading swap calldata, enforcing `minAmountOut` dynamically against configured tolerances.
7. **`DailySpendLimit`**: Resets at the daily UNIX boundary, tracking cumulative spend and reverting any atomic transactions that naturally push over the dollar limit threshold.

### Six-Layer Defense Architecture

| Layer | Mechanism | Bypassed by Prompt Injection? |
|---|---|---|
| 1. Prompt-level | Safety instructions in system prompt | Yes (prompt injection defeats this) |
| 2. Type-system | `Capability<T>` tokens in Rust | **No** (compiler-enforced) |
| 3. On-chain | PolicyCage smart contract (Solidity revert) | **No** (EVM-enforced) |
| 4. Hardware | TEE/enclave for key material | **No** (hardware boundary) |
| 5. Runtime | Rate limiting, anomaly detection, conductor watchers | Maybe (depends on integrity) |
| 6. Off-chain | Monitoring, alerting, human review | Informational only |

**The guarantee**: Layers 2 and 3 are outside the LLM's reach. Even if the model is completely compromised (Anthropic reports ~12% prompt injection bypass rate), it cannot forge capability tokens (Rust prevents it) and cannot exceed on-chain spending caps (EVM reverts the transaction). Safety is architectural, not behavioral.

---

## The Conductor (Reactive Intelligence)

10 specialized watchers monitor agent behavior and emit intervention signals:

| Watcher | What It Detects | Escalation |
|---|---|---|
| `CompileFailRepeat` | 2+ consecutive compile failures | Warn → restart with error digest |
| `ContextWindowPressure` | Token consumption approaching limit | Warn → trigger compaction |
| `CostOverrun` | USD spent exceeding budget | Warn → downgrade model → halt |
| `GhostTurn` | Turn with zero tool calls | Warn → restart with nudge |
| `IterationLoop` | Same task repeated N times | Warn → abort task |
| `ReviewLoop` | Review-edit-review without progress | Warn → skip review |
| `SpecDrift` | Output diverging from spec | Warn → re-read spec |
| `StuckPattern` | Generic stuck detection | Warn → restart |
| `TestFailureBudget` | Consecutive test failures | Warn → abort after budget |
| `TimeOverrun` | Phase duration exceeded | Warn → escalate or abort |

**Circuit Breaker**: Tracks cumulative failures per plan. Trips on threshold: warn → throttle → halt. Prevents throwing good money after bad.

**ConductorDecision**:
```rust
pub enum ConductorDecision {
    Proceed,                                    // Continue normally
    Warn(String),                               // Log warning, continue
    Restart { reason: String, model_hint: Option<String> },  // Restart with different model
    Abort { reason: String },                   // Halt execution
}
```

**Research**: Cybernetic control theory (Ashby, 1956). The conductor is a second-order regulator — it regulates the agent, which regulates the codebase. Good Regulator Theorem (Conant & Ashby, 1970) — the conductor must model the agent's behavior to regulate it effectively.

---

## The Extension System (roko-plugin)

### Two Extension Points

**EventSource**: Continuously emits signals until cancellation.

```rust
pub trait EventSource: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn kind(&self) -> EventSourceKind;  // Webhook, Cron, FileWatch, Custom
    async fn start(&self, sender: SignalSender, cancel: CancellationToken) -> Result<()>;
}
```

Built-in implementations:
- **CronEventSource**: Scheduled signal emission (cron expressions from config)
- **FileWatchEventSource**: Filesystem change detection (recursive, debounced, glob-filtered)

**FeedbackCollector**: Periodically polls external services for feedback.

```rust
pub trait FeedbackCollector: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn services(&self) -> Vec<String>;
    fn interval(&self) -> Duration;
    async fn collect(&self, since: DateTime<Utc>) -> Result<Vec<FeedbackSignal>>;
}
```

`FeedbackSignal` carries: `original_episode_id`, `service`, `outcome` (Approved/Rejected/Commented/Merged/Ignored), `metadata`, `timestamp`.

### MCP Servers as Plugins

Four MCP server crates extend roko's tool surface:

| Server | What It Provides |
|---|---|
| `roko-mcp-github` | GitHub API tools (issues, PRs, comments, users) |
| `roko-mcp-slack` | Slack Web API tools (messages, channels, threads, files) |
| `roko-mcp-scripts` | Generic script→tool wrapper (any shell script becomes a tool) |
| `roko-mcp-stdio` | Shared JSON-RPC stdio transport layer |

Each server is a standalone binary that speaks MCP over stdio. Configured in `roko.toml`:

```toml
[[mcp]]
name = "github"
command = "cargo run --release -p roko-mcp-github"
```

The agent discovers tools via MCP's `tools/list` and calls them like any built-in tool. The `DynamicToolRegistry` merges MCP tools with built-in tools transparently.

---

## The 28 Agent Roles (Complete Taxonomy)

### Role Groups

| Group | Roles | Default Backend | Purpose |
|---|---|---|---|
| **Meta** | Conductor | Claude (Haiku) | Orchestration, routing, no code writes |
| **Coding** | Implementer, AutoFixer, Strategist, Researcher | Claude | Code writing, planning, research |
| **Review** | Architect, Auditor, QuickReviewer, Scribe, Critic | Claude/Codex | Code review, documentation |
| **Testing** | IntegrationTester, CrossSystemTester, TerminalValidator, FullLoopValidator, RegressionDetector, PerformanceSentinel, CoverageTracker, SpecDriftDetector | Codex | Verification at various levels |
| **Utility** | PrePlanner, ErrorDiagnoser, DependencyValidator, PatternExtractor, SnapshotComparator, MergeResolver, Refactorer, DocVerifier, PlanLifecycleManager | Codex | Specialized support tasks |

### Tool Permission Matrix (Principle of Least Privilege)

| Role | Read | Glob | Grep | Edit | Write | Bash | Web | JSON Schema | MCP |
|---|---|---|---|---|---|---|---|---|---|
| Conductor | ✓ | - | - | - | - | - | - | - | ✓ |
| Implementer | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | (effort-gated) | - | ✓ |
| AutoFixer | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - | - | - |
| Strategist | ✓ | ✓ | ✓ | - | - | ✓ | ✓ | - | ✓ |
| Researcher | ✓ | ✓ | ✓ | - | - | ✓ | ✓ | - | ✓ |
| Architect | ✓ | ✓ | ✓ | - | - | ✓ | ✓ | ✓ | ✓ |
| Auditor | ✓ | ✓ | ✓ | - | - | ✓ | ✓ | ✓ | ✓ |
| Scribe | ✓ | ✓ | ✓ | - | ✓ | - | ✓ | - | ✓ |
| MergeResolver | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - | - | - |
| Refactorer | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - | - | - |
| All testers | ✓ | ✓ | ✓ | - | - | ✓ | - | - | - |
| All utility | ✓ | ✓ | ✓ | - | - | ✓ | - | - | - |

**Key design**: Each role gets exactly the tools it needs. Fewer tools = model wastes fewer tokens considering unavailable tools = higher accuracy + lower cost. The Implementer has 9 tools; the Conductor has 2.

### Complexity-Based Pipeline Selection

| Complexity | Strategist | Reviews | Quick Review | Critic | Max Iterations |
|---|---|---|---|---|---|
| **Trivial** | Skip | Skip | No | No | 1 |
| **Simple** | Skip | Skip | No | No | 2 |
| **Standard** | Skip | Yes | Yes (single) | No | 2 |
| **Complex** | Skip | Yes | No (full: Architect+Auditor+Scribe) | Yes | 2 |

Trivial tasks skip the entire review pipeline. Complex tasks get 3 parallel reviewers + a critic. The system doesn't waste $3 reviewing a one-line config change.

---

## The Full DeFi Tool Surface (423+ Tools)

Beyond the 16 built-in coding tools, roko's DeFi agent mode provides **423+ specialized tools** across 17 categories:

| Category | Tool Count | Examples |
|---|---|---|
| **Data** | ~50 | Price feeds, pool state, liquidity depth, APY, OHLCV history |
| **Trading** | ~40 | Swaps, limit orders, UniswapX, EIP-7702 delegation |
| **LP Management** | ~35 | Add/remove liquidity, V3/V4 ranges, fee collection |
| **Lending** | ~40 | Aave V3, Morpho Blue, Fluid, Moonwell, Seamless |
| **Staking** | ~30 | Lido, Rocket Pool, cbETH, StakeWise; LST tracking |
| **Restaking** | ~20 | EigenLayer, Symbiotic; LRT depeg detection |
| **Derivatives** | ~25 | GMX v2, Hyperliquid, Synthetix v3 perps; Panoptic options |
| **Yield** | ~20 | Yearn V3, Beefy, Convex, Aura aggregation |
| **Bridge** | ~25 | Cross-chain quotes, ERC-7683 intents, status tracking |
| **Vault** | ~20 | ERC-4626 deposit/withdraw, yield tracking |
| **Curve/Balancer** | ~15 | Pool state, LP, gauge voting |
| **CDP/Stablecoins** | ~15 | MakerDAO/Sky, Liquity v2, crvUSD, FraxLend |
| **DEX Aggregators** | ~15 | 1inch, CoW Protocol, Paraswap, Odos |
| **Safety** | ~20 | Health factor monitoring, MEV risk, IL calculation |
| **Intelligence** | ~15 | Venue comparison, token discovery, points strategies |
| **Memory/Identity** | ~12 | Utility, streaming, wallet management |
| **Config** | ~10 | Profile configuration, distribution |

### Trust Tiers (Compile-Time Enforcement)

| Tier | Tools | Capability Required | What It Means |
|---|---|---|---|
| **ReadTool** | ~250 | None | Can read any on-chain state |
| **WriteTool** | ~150 | `Capability<WriteTool>` (consumed on use) | Can modify positions, execute trades |
| **PrivilegedTool** | ~23 | `Capability<Self>` + owner approval | Can change PolicyCage params, strategy |

### Progressive Disclosure: 10 Tool Profiles

Agents don't load all 423 tools. They load a **profile** that matches their strategy:

1. **Read-Only Observatory**: All read tools, no writes
2. **Portfolio Monitor**: Read + health monitoring
3. **Trader**: Read + basic swaps
4. **LP Manager**: Read + LP operations + fee collection
5. **Yield Optimizer**: LP + lending + staking + aggregators
6. **Full Access**: All 423+ tools

The LLM never sees tools it can't use. This reduces prompt size and improves accuracy (fewer irrelevant tools = better tool selection).

### Two-Layer Tool Model

What the LLM sees: **8 Pi-facing tools** (`preview_action`, `commit_action`, `query_state`, etc.)
What actually executes: **423+ tool implementations**

The Pi-facing tools are high-level intents ("I want to swap ETH for USDC"). The tool router resolves the intent to the specific implementation (Uniswap V3 swap via 1inch aggregator on Base).

---

## What's Unique About This Stack

| Component | Standard Approach | Roko's Approach | Why It's Better |
|---|---|---|---|
| **Tool dispatch** | Direct function call | 7-step pipeline with safety + auth | Prevents unauthorized tool use even under prompt injection |
| **Concurrency** | All tools run sequentially | Parallel/serial partitioning by concurrency policy | 3-5x faster for read-heavy workloads |
| **Safety** | Runtime prompt guardrails | Compile-time capability tokens | Cannot be bypassed — enforced by Rust's type system |
| **Monitoring** | Log-based after the fact | Real-time conductor with 10 watchers | Sub-second anomaly detection, automatic intervention |
| **Extensions** | Plugin architecture varies | Two traits (EventSource + FeedbackCollector) + MCP | Simple to implement, object-safe, async-native |
| **Process management** | Ad-hoc subprocess handling | ProcessSupervisor with hierarchical cancellation | Clean shutdown, no orphan processes, budget-aware |
| **Tool surface** | Fixed set | 16 built-ins + unlimited MCP tools | Extensible without modifying core, discoverable at runtime |

---

## The Safety Architecture: Defense in Depth

Security for autonomous agents cannot rely on any single mechanism. A prompt injection can defeat prompt-level guardrails. A logic bug can bypass runtime checks. A compromised dependency can subvert process isolation. The only defensible architecture is **defense in depth** — six independent layers, each sufficient to prevent catastrophic failure even if all other layers are compromised.

### Layer 1: Capability Tokens (`Capability<T>`)

The foundation of the safety architecture is a Rust type that cannot be forged, cloned, or reused:

```rust
pub struct Capability<T> {
    _marker: PhantomData<T>,
    expiry: Instant,
    nonce: u64,
    // pub(crate) constructor — cannot be created outside the safety crate
    // no Clone, no Copy — consumed by value on use
    // no Default — cannot be conjured from nothing
}
```

**Single-use semantics enforced by Rust's ownership model**: When a `Capability<WriteTool>` is passed to a tool's `execute()` method, it is moved (consumed). The caller no longer has it. There is no way to use it again — the compiler rejects the code at compile time, not at runtime.

**Time-bounded**: Each capability carries an `expiry` timestamp. The safety layer checks `Instant::now() < expiry` before accepting the token. Stale capabilities are rejected even if structurally valid.

**Typed**: `Capability<ReadTool>`, `Capability<WriteTool>`, and `Capability<PrivilegedTool>` are distinct types. Rust's type system prevents passing a read capability where a write capability is required. There is no casting, no downcasting, no dynamic dispatch that could confuse them.

**Nonce-tagged**: Each capability carries a monotonically increasing nonce. The safety layer tracks consumed nonces and rejects duplicates — defense against replay in the (impossible, but defended-against) event of memory corruption.

### Layer 2: Process Isolation via Git Worktrees

Each agent runs in its own git worktree — a full filesystem checkout with its own working tree, index, and HEAD. This provides:

- **Filesystem isolation**: Agent A cannot read or write Agent B's files. Path traversal attacks (`../../agent-b/secrets`) resolve to a location outside the worktree, which the Path Policy rejects via canonicalization.
- **Git isolation**: Agent A's commits, branches, and staging area are independent. One agent's `git reset --hard` cannot destroy another agent's work.
- **Environment isolation**: Each worktree has its own `CARGO_TARGET_DIR`, preventing build cache collisions between concurrent agents.
- **Kill isolation**: Each agent process runs in its own process group (`setpgid(0, 0)`). Killing an agent kills only its descendants, not other agents.

The worktree is the security boundary. Everything inside it is the agent's sandbox. Everything outside it is inaccessible.

### Layer 3: Taint Tracking (`TaintedString` / `CleanString`)

All data entering the system from untrusted sources (LLM outputs, tool results, user inputs, network responses) is wrapped in `TaintedString`:

```rust
pub struct TaintedString(String);  // Cannot be used where CleanString is expected
pub struct CleanString(String);    // Can only be created via explicit sanitization

impl TaintedString {
    pub fn sanitize(self, policy: &SanitizationPolicy) -> CleanString {
        // Apply policy-specific sanitization rules
        // Strip control characters, validate UTF-8, apply allow/deny patterns
        CleanString(policy.apply(&self.0))
    }
}
```

**The type system prevents accidental trust**: Functions that construct shell commands, file paths, or SQL queries accept `CleanString`, not `String` or `TaintedString`. The compiler rejects any code path that passes unsanitized data to a sensitive operation.

**Sanitization policies are explicit**: Each use site specifies what kind of sanitization is needed. A `PathSanitizationPolicy` differs from a `BashSanitizationPolicy` differs from a `LogSanitizationPolicy`. There is no universal "make it safe" function — that would be a false sense of security.

**Taint propagation**: When tainted data is concatenated with clean data, the result is tainted. Taint only flows in one direction — toward distrust. Only explicit sanitization can remove it.

### Layer 4: Audit Chain (SHA-256 Hash-Linked Log)

Every safety-relevant event is appended to a tamper-evident log:

```rust
pub struct AuditEntry {
    pub seq: u64,                    // Monotonic sequence number
    pub timestamp: DateTime<Utc>,
    pub prev_hash: [u8; 32],        // SHA-256 of previous entry
    pub event: AuditEvent,          // One of 11 event types
    pub actor: ActorId,             // Which agent or system component
    pub hash: [u8; 32],             // SHA-256(seq || prev_hash || event || actor)
}
```

**11 event types**: `CapabilityIssued`, `CapabilityConsumed`, `CapabilityExpired`, `ToolCallAuthorized`, `ToolCallDenied`, `PolicyViolation`, `SafetyOverride`, `CircuitBreakerTrip`, `AgentSpawned`, `AgentKilled`, `ConstitutionVerified`.

**Hash-linked append-only**: Each entry's `hash` includes the previous entry's hash. Tampering with any entry invalidates all subsequent hashes. `verify()` walks the chain from genesis and checks every link.

**On-chain anchoring**: Every 1,000 ticks (approximately every 6.7 minutes at 400ms block times), the chain head hash is committed to the Korai Ledger contract. This creates an externally verifiable timestamp — even if the local log is destroyed, the on-chain anchor proves what the state was at that point.

**Forensic utility**: After an incident, the audit chain provides a complete, ordered, tamper-evident record of every safety-relevant decision. Which capabilities were issued, to whom, when they were consumed, which tool calls were authorized or denied, and which policy violations occurred.

### Layer 5: Constitution (`ConstitutionConfig`)

The constitution is the set of immutable safety parameters that define the agent's operating envelope:

```rust
pub struct ConstitutionConfig {
    pub max_spend_per_tx_usd: f64,
    pub max_spend_per_day_usd: f64,
    pub approved_protocols: Vec<Address>,
    pub approved_assets: Vec<Address>,
    pub max_slippage_bps: u16,
    pub sanctions_oracle: Address,
    pub emergency_withdrawal_only: bool,
    // ... additional parameters
}
```

**Verification at boot**: At startup, the agent computes `keccak256(abi.encode(constitution_params))` and compares it to the hash stored on-chain in the PolicyCage contract. If they differ — if someone modified the local config file, or if the on-chain parameters were updated without the agent's knowledge — the agent **refuses to start**. No silent degradation, no fallback mode, no override.

**Why this matters**: A sophisticated attacker could modify the agent's local configuration to relax safety limits. The constitution check ensures that the agent's understanding of its limits matches the on-chain enforcement. The two must agree, or nothing runs.

### Layer 6: Loop Guard (`LoopGuard` / `LoopVerdict`)

Autonomous agents can enter infinite loops — repeating the same failed action, cycling between two states, or generating endless intermediate results without converging:

```rust
pub struct LoopGuard {
    history: VecDeque<ActionFingerprint>,
    max_window: usize,           // How many recent actions to track
    repeat_threshold: usize,     // How many repeats before triggering
    similarity_threshold: f64,   // Hamming distance threshold for "similar enough"
}

pub enum LoopVerdict {
    Continue,                    // No loop detected, proceed normally
    Warning(String),             // Possible loop, inject a nudge
    ForceTerminate(String),      // Definite loop, kill the agent
}
```

**Fingerprinting**: Each agent action (tool call + arguments + result summary) is hashed into an `ActionFingerprint`. The LoopGuard maintains a sliding window of recent fingerprints and computes pairwise similarity.

**Escalation**: First detection emits a `Warning` — the conductor injects a nudge into the next turn ("You appear to be repeating the same action. Try a different approach."). If the loop continues past the threshold, `ForceTerminate` kills the agent and logs the loop pattern for post-mortem analysis.

**Why separate from the conductor**: The conductor's 10 watchers detect behavioral patterns (compile failures, stuck turns, cost overruns). The LoopGuard is a lower-level mechanism that operates on raw action fingerprints, catching loops that the conductor's pattern matchers might miss.

### The Layered Guarantee

| Layer | What It Prevents | Bypass Requires |
|---|---|---|
| 1. Capability tokens | Unauthorized tool execution | Rust compiler exploit |
| 2. Process isolation | Cross-agent interference | OS kernel exploit |
| 3. Taint tracking | Injection attacks (path, shell, SQL) | Rust type system exploit |
| 4. Audit chain | Evidence tampering | SHA-256 collision |
| 5. Constitution | Parameter drift | On-chain contract exploit |
| 6. Loop guard | Infinite agent loops | Adversarial loop disguise |

An attacker would need to simultaneously exploit the Rust compiler, the OS kernel, the SHA-256 hash function, and the EVM to fully compromise the safety architecture. No single failure — including complete LLM compromise via prompt injection — can breach more than one layer.

**Research**: Capability-Based Security (Dennis & Van Horn, 1966) — the theoretical foundation for Layer 1. OWASP Top 10 for LLM Applications (2023) — practical attack taxonomy that informed the layered defense design. Defense in depth (NSA, 2010) — the architectural principle that no single control is trusted.

---

## The Tool Taxonomy: Trust Tiers

### Three Trust Levels Enforced by Rust's Type System

The 423+ tools in roko's DeFi surface are not all created equal. A price query and a fund withdrawal carry fundamentally different risk profiles. The type system makes this distinction at compile time, not runtime:

**`ReadTool`**: Pure observation. These tools can gather information but cannot change any state — on-chain or off-chain.

- Balance checks (`get_balance`, `get_portfolio_value`)
- Price queries (`get_token_price`, `get_pool_state`, `get_ohlcv`)
- Position inspection (`get_lp_position`, `get_lending_health`, `get_staking_rewards`)
- Protocol state (`get_apy`, `get_tvl`, `get_liquidity_depth`)
- **No capability token required**. Read operations are always safe. An agent can query freely without consuming any safety budget.

**`WriteTool`**: Modifies state. These tools execute transactions or alter the agent's local state.

- Swaps (`execute_swap`, `execute_limit_order`)
- LP management (`add_liquidity`, `remove_liquidity`, `collect_fees`)
- Lending (`deposit_collateral`, `borrow`, `repay`, `withdraw`)
- Staking (`stake`, `unstake`, `claim_rewards`)
- **Requires a `Capability<WriteTool>` token**: Single-use, consumed on execution. The safety layer issues one token per authorized action. After execution, the token is gone — the agent must request a new one for the next write.

**`PrivilegedTool`**: System-critical operations that could cause catastrophic loss if misused.

- Wallet management (`export_key`, `rotate_session_key`, `revoke_delegation`)
- Policy changes (`update_approved_assets`, `modify_spending_cap`, `change_slippage_limit`)
- Emergency operations (`emergency_withdrawal`, `pause_all_trading`, `kill_agent`)
- Strategy modification (`change_strategy_type`, `modify_risk_parameters`)
- **Requires elevated `Capability<PrivilegedTool>` + operator approval**: The agent cannot issue these capabilities to itself. An operator must explicitly approve each privileged operation through the ActionPermit flow. The approval is time-bounded and nonce-tagged.

### Progressive Disclosure via 10 Tool Profiles

Agents do not see all 423+ tools. They see only the tools relevant to their current strategy, loaded via one of 10 **tool profiles**:

| Profile | Read Tools | Write Tools | Privileged Tools | Use Case |
|---|---|---|---|---|
| 1. Read-Only Observatory | All (~250) | None | None | Market analysis, research |
| 2. Portfolio Monitor | All + health alerts | None | None | Passive monitoring |
| 3. DCA Accumulator | Price + balance | Basic swap only | None | Dollar-cost averaging |
| 4. Trader | Market data + orderbook | Swaps + limit orders | None | Active trading |
| 5. LP Manager | Pool state + position | LP operations + fees | None | Liquidity provision |
| 6. Yield Optimizer | All read + APY | LP + lending + staking | None | Yield farming |
| 7. Lending Specialist | Health factors + rates | Lending operations | None | Supply/borrow management |
| 8. Bridge Operator | Cross-chain state | Bridge execution | None | Cross-chain transfers |
| 9. Strategy Manager | All read | All write | Strategy modification | Full strategy control |
| 10. Full Access | All (~250) | All (~150) | All (~23) | Operator-level access |

**Why this matters**: A DCA accumulator agent that only needs to check prices and execute periodic swaps has no business seeing liquidation tools, bridge tools, or strategy modification tools. Fewer visible tools means:
- **Smaller prompts**: 30 tools instead of 423 = dramatic token savings
- **Better tool selection**: The model chooses from a focused set, reducing confusion
- **Reduced attack surface**: A compromised DCA agent literally cannot call `emergency_withdrawal` — the tool is not in its registry

### The Two-Layer Tool Model

The 423+ implementation tools are not what the LLM sees. The LLM interacts with **8 high-level intent tools** that map to the underlying implementations:

| LLM-Facing Intent Tool | What It Resolves To | Example |
|---|---|---|
| `analyze_opportunity` | 50+ data tools | Pool state + price history + APY comparison |
| `execute_trade` | 40+ trading tools | Best-route swap via 1inch/CoW/Paraswap |
| `manage_position` | 35+ LP tools | Rebalance V3 range, collect fees |
| `manage_lending` | 40+ lending tools | Deposit to Morpho, adjust collateral ratio |
| `check_safety` | 20+ safety tools | Health factor check, MEV risk assessment |
| `manage_staking` | 30+ staking tools | Stake ETH to Lido, claim rewards |
| `bridge_assets` | 25+ bridge tools | ERC-7683 intent for cross-chain transfer |
| `query_state` | All read tools | Any state query, routed by parameter type |

**The LLM reasons about intents, not raw calldata**. When the agent says "execute a swap of 1 ETH for USDC with max 0.5% slippage," the intent tool resolves this to: query 1inch for best route → simulate via Revm → check slippage against PolicyCage → encode calldata → submit transaction. The LLM never sees the ABI encoding, the route optimization, or the gas estimation — those are implementation details handled by the TypeScript sidecar and the Rust runtime.

**Why two layers**: LLMs excel at high-level reasoning ("I should swap now because the price is favorable") but struggle with low-level details ("encode `exactInputSingle` with `sqrtPriceLimitX96` set to the appropriate Q64.96 value"). The two-layer model lets each system do what it does best.

**Research**: Intent-centric architecture (ERC-4337, ERC-7683) — the same principle applied to user transactions. The user expresses intent; the infrastructure resolves execution. Roko applies this to agent-tool interaction.
