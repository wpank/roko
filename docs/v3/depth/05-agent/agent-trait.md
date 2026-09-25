# 05-agent/agent-trait -- The Agent Trait

> Full specification of the `Agent` trait, `AgentResult`, streaming extension,
> lineage helpers, and the actor-model foundations that inform the design.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-agent/src/agent.rs`

---

## 1. Why Agents Are Separate from the Kernel Traits

Roko's kernel exposes 12 composable traits that process Signals. These traits
share four properties: they are synchronous, deterministic (given fixed inputs),
side-effect-free, and they process single Signals at a time.

An **Agent** violates all four:

| Kernel property | Agent behavior |
|-----------------|---------------|
| Synchronous | Async -- spawns subprocesses, calls LLM APIs, awaits network |
| Deterministic | Stochastic -- same prompt produces different outputs each run |
| Side-effect-free | Side-effecting -- edits files, runs commands, mutates state |
| Single-signal | Multi-signal -- produces a stream of tool calls, diffs, status before final output |

Rather than distort a kernel trait (e.g. making `Compose` async and side-effecting),
`Agent` is its own capability extension. The kernel stays clean and testable; agent
implementations live in `roko-agent`.

This separation follows the CoALA cognitive architecture (Sumers et al., 2023,
arXiv:2309.02427), which draws a firm boundary between perception/reasoning
(the kernel traits) and action execution (agents).

---

## 2. The Agent Trait

The trait lives at `crates/roko-agent/src/agent.rs` and has five methods, three
of which have default implementations:

```rust
// crates/roko-agent/src/agent.rs

#[async_trait]
pub trait Agent: Send + Sync {
    /// Run the agent against the input signal.
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult;

    /// Human-readable name for logs/metrics.
    fn name(&self) -> &str;

    /// Stable backend identifier for audit and episode logging.
    fn backend_id(&self) -> &'static str { "unknown" }

    /// Does this agent emit a streaming trace or a single output?
    fn supports_streaming(&self) -> bool { false }

    /// Run the agent with streaming output.
    /// Default: falls back to `run()` and emits a single TextDelta.
    async fn run_streaming(
        &self,
        input: &Signal,
        ctx: &Context,
        event_tx: mpsc::Sender<StreamEvent>,
    ) -> AgentResult { /* default impl */ }
}
```

### Design decisions

- **`Send + Sync`** -- Required because the runtime runs agents across `tokio`
  tasks. Every concrete implementation must be thread-safe.

- **`&Signal` input** -- Borrowed, not consumed. The runtime keeps the original
  prompt signal for logging and DAG lineage while the agent works with a reference.

- **`&Context` context** -- Carries a timestamp and runtime metadata. Clean injection
  point without polluting the trait signature.

- **`AgentResult` return** -- Not `Result<T, E>`. Agents always return an
  `AgentResult` that wraps success/failure as a boolean flag, because even
  "failed" agent runs produce diagnostic output that the runtime needs for
  logging and retry decisions.

- **`backend_id()`** -- Stable identifier (e.g., `"anthropic_api"`, `"claude_cli"`)
  for audit trails and episode attribution. Defaults to `"unknown"` for
  implementations that do not override it.

- **`run_streaming()`** -- The default implementation calls `run()`, then emits
  a single `StreamEvent::TextDelta` with the full output text plus a `Usage`
  event. Providers that support real streaming (Anthropic SSE, OpenAI SSE)
  override this to forward events as they arrive.

---

## 3. AgentResult

The result of running an agent once:

```rust
pub struct AgentResult {
    pub output: Signal,                    // Primary output (Kind::AgentOutput)
    pub trace: Vec<Signal>,                // Intermediate signals, chronological
    pub usage: Usage,                      // Legacy token counts, cost, duration
    pub usage_obs: Option<UsageObservation>, // Canonical usage with provenance
    pub success: bool,                     // false on non-zero exit / connection error
}
```

### Constructors and builders

| Method | Purpose |
|--------|---------|
| `AgentResult::ok(output)` | Successful result with just an output signal |
| `AgentResult::fail(output)` | Failed result with diagnostic output |
| `.with_trace(vec)` | Attach intermediate signals |
| `.with_usage(usage)` | Attach legacy usage; mirrors into `usage_obs` |
| `.with_usage_obs(obs)` | Attach canonical usage; derives legacy counters |
| `.all_signals()` | Returns `trace` then `output` in chronological order |

The `all_signals()` ordering matters: episode logging writes each signal as a
row in `.roko/episodes.jsonl`, and chronological order is required for replay.

### Dual usage representation

`AgentResult` carries both `usage: Usage` (the legacy flat struct) and
`usage_obs: Option<UsageObservation>` (the canonical observation with optional
provenance metadata). The `with_usage()` builder mirrors the legacy struct into
`usage_obs`; the `with_usage_obs()` builder derives the legacy counters from the
canonical observation. This dual representation exists because the codebase has
many consumers of the flat `Usage` struct, and migration is incremental.

---

## 4. Lineage Helpers

Two free functions assist with signal lineage propagation:

```rust
/// Build an output signal that keeps the full upstream lineage from `input`.
pub fn derived_output(input: &Signal, kind: Kind, body: Body) -> SignalBuilder;

/// Return the full upstream lineage for `input`, including the input hash.
pub fn full_lineage(input: &Signal) -> impl Iterator<Item = ContentHash> + '_;
```

`derived_output` centralizes the "input lineage + direct parent" rule: the
output signal's `lineage` field contains all of the input's ancestors plus the
input itself. This ensures that the signal DAG can be traversed from any output
back to the original prompt.

---

## 5. Where the Agent Sits in the Kernel Loop

In the universal cognitive loop -- query, score, route, compose, **act**, verify,
write, react -- the Agent occupies the **act** step. It is the bridge between
the pure kernel world and the impure, side-effecting real world.

```
query -> score -> route -> compose -> [Agent.run()] -> verify -> write -> react
                                          ^
                                     side effects
                                     async I/O
                                     non-determinism
```

The kernel traits handle everything before and after the Agent. The Agent is the
single point where control crosses from the deterministic domain into the
stochastic domain and back.

---

## 6. Concrete Implementations

Roko ships 12 provider-backed agent implementations, each targeting a different
backend protocol. All are constructed through `adapter_for_kind()` at
`crates/roko-agent/src/provider/mod.rs`:

| Kind | Agent struct | Protocol | Tool loop |
|------|-------------|----------|-----------|
| `AnthropicApi` | `AnthropicApiAgent` | HTTP (Messages API) | Roko ToolLoop |
| `ClaudeCli` | `ClaudeCliAgent` | Stream-JSON subprocess | Internal (Claude CLI) |
| `CodexCli` | `CodexCliAgent` | Subprocess (`codex exec --json`) | Internal (Codex) |
| `OpenAiCompat` | `OpenAiCompatAgent` | HTTP (`/v1/chat/completions`) | Roko ToolLoop |
| `CursorAcp` | `CursorAcpAgent` | ACP JSON-RPC | Internal |
| `CursorCli` | `CursorCliAgent` | ACP JSON-RPC over stdio | Internal |
| `PerplexityApi` | `PerplexityApiAgent` | HTTP (Sonar extensions) | Roko ToolLoop |
| `GeminiApi` | `GeminiApiAgent` | HTTP (Gemini native) | Roko ToolLoop |
| `GeminiCli` | `GeminiCliAgent` | Subprocess (`gemini`) | Internal |
| `CerebrasApi` | `CerebrasApiAgent` | HTTP (OpenAI-compat) | Roko ToolLoop |
| `Hermes` | `HermesAgent` | HTTP/CLI/ACP | Varies |
| `OpenClaw` | `OpenClawAgent` | CLI/ACP | Internal |

Additionally, `MockAgent` returns predetermined responses for unit testing.

---

## 7. Actor Model Foundations

The `Agent` trait's design is rooted in the actor model (Hewitt et al., 1973):

| Actor model concept | Roko equivalent |
|---------------------|-----------------|
| Actor | `Box<dyn Agent>` |
| Message | `Signal` |
| Behavior | `AgentRole` + system prompt |
| Supervision tree | `PlanRunner` + `ProcessSupervisor` |
| Let-it-crash | Gate pipeline: fail -> retry with fallback model |
| Behavior switching | Lifecycle type-state transitions (`SlotManager`) |

The supervision model maps to Erlang/OTP restart strategies:

- **one_for_one** -- Restart only the failing task (retry with fallback model)
- **rest_for_one** -- Re-run downstream DAG tasks when upstream fails
- **one_for_all** -- Re-run entire plan on critical failure

The `ProcessSupervisor` in `roko-runtime` handles subprocess lifecycle: spawn,
monitor exit codes/stdout/stderr, SIGTERM -> wait -> SIGKILL for graceful shutdown.

---

## 8. StreamEvent Types

The streaming extension uses `StreamEvent` from `crates/roko-agent/src/tool_loop/`:

| Event kind | Payload | When emitted |
|------------|---------|-------------|
| `TextDelta` | Incremental text chunk | During streaming response |
| `ToolCallStart` | Tool call ID + name | When model requests a tool |
| `ToolCallDelta` | Incremental arguments | During streaming tool args |
| `ToolResult` | Result content | After tool execution |
| `Usage` | Token counts + cost | At response completion |
| `Error` | Error message | On backend failure |

The `event_tx: mpsc::Sender<StreamEvent>` channel decouples the agent's
production of events from the consumer's rate of processing, enabling
backpressure-aware streaming.

---

## 9. Test Coverage

The trait module includes unit tests verifying:

- `AgentResult::ok` sets `success = true` with empty trace
- `AgentResult::fail` sets `success = false`
- `all_signals()` returns trace-then-output in order
- Builder chain: `.with_trace()` then `.with_usage()` preserves all fields
- `with_usage()` mirrors into `usage_obs`

---

## 10. Citations

1. Sumers, T. R. et al. (2023). "Cognitive Architectures for Language Agents."
   arXiv:2309.02427. -- CoALA: separating perception/reasoning from action.
2. Hewitt, C., Bishop, P., & Steiger, R. (1973). "A Universal Modular ACTOR
   Formalism for Artificial Intelligence." IJCAI. -- Actor model foundation.
3. `crates/roko-agent/src/agent.rs` -- Agent trait, AgentResult, lineage
   helpers source.
4. `crates/roko-runtime/src/supervisor.rs` -- ProcessSupervisor for subprocess
   lifecycle management.
5. `crates/roko-core/src/agent.rs` -- ProviderKind enum, 12 variants.
