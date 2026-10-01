# 05-agent/tool-loop-protocol -- Tool Loop Protocol

> The ToolLoop multi-turn driver, LlmBackend trait, stop conditions,
> checkpoint/resume, context pruning, and the 7-step ToolDispatcher pipeline.
> Implements the ReAct pattern (Yao et al., 2023, arXiv:2210.03629, ICLR 2023).

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** `crates/roko-agent/src/tool_loop/mod.rs`,
`crates/roko-agent/src/dispatcher/mod.rs`

---

## 1. The ReAct Pattern

The ToolLoop implements the **ReAct** pattern (Yao et al., 2023, "ReAct:
Synergizing Reasoning and Acting in Language Models," arXiv:2210.03629,
ICLR 2023). ReAct interleaves reasoning traces ("Thought") with task-specific
actions ("Action") and environment feedback ("Observation"):

```
Thought: I need to find the configuration file.
Action: read_file("/src/config.rs")
Observation: [file contents]
Thought: The struct is missing a field. I should add it.
Action: edit_file("/src/config.rs", ...)
Observation: [edit result]
Thought: Now I need to update the tests.
...
```

The ToolLoop abstracts this pattern: the LLM reasons, picks tools, gets results,
and continues until it produces a final answer or hits a stop condition.

---

## 2. Core Cycle

```
prompt --> LLM --> tool_calls? --> dispatch --> results --> LLM --> ...
```

The loop runs until one of four stop conditions:

| Condition | Meaning |
|-----------|---------|
| **Stop** | LLM returns a response with no tool calls (final answer) |
| **MaxIterations** | Iteration cap reached (default: 25) |
| **Cancelled** | Cancel token tripped between turns |
| **BackendError** | LLM returns an error |

---

## 3. LlmBackend Trait

The interface between the ToolLoop and the LLM:

```rust
pub trait LlmBackend: Send + Sync {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
    ) -> Result<BackendResponse, LlmError>;
}
```

This is intentionally lower-level than the `Agent` trait:

- `Agent::run()` models a complete agent run (potentially many turns)
- `LlmBackend::send_turn()` models a single request-response round

The ToolLoop calls `send_turn()` once per iteration, inspects the response for
tool calls via the `Translator`, dispatches through the `ToolDispatcher`,
formats results, and calls `send_turn()` again.

### LlmError

```rust
pub enum LlmError {
    Backend(String),  // API error, non-success status
    Network(String),  // DNS, timeout, connection reset
}
```

---

## 4. ToolLoop Construction

```rust
pub struct ToolLoop {
    translator: Arc<dyn Translator>,
    dispatcher: Arc<ToolDispatcher>,
    backend: Arc<dyn LlmBackend>,
    max_iterations: usize,         // Default: 25
    context_token_limit: usize,    // From prune module
}
```

Three dependencies, all injected via `Arc`:

1. **Translator** -- Converts between canonical tools and the backend's wire
   format. Selected based on `ModelProfile::tool_format`.
2. **ToolDispatcher** -- Runs tool calls through the safety + execution pipeline.
3. **LlmBackend** -- Sends conversation turns to the LLM.

---

## 5. The Core Loop (Pseudocode)

```rust
loop {
    // 1. Check iteration cap
    if max_iter::is_exhausted(iterations, self.max_iterations) {
        return checkpoint + MaxIterations;
    }

    // 2. Check cancellation
    if ctx.is_cancelled() {
        return checkpoint + Cancelled;
    }

    // 3. Send turn to LLM
    let response = self.backend.send_turn(&messages, &rendered_tools).await?;

    // 4. Parse tool calls
    let calls = self.translator.parse_calls(&response)?;

    // 5. No tool calls -> final answer
    if calls.is_empty() {
        return ToolLoopOutput { final_text: response.extract_text(), ... };
    }

    // 6. Inject assistant message into history
    messages.push(self.translator.render_assistant_message(&response));

    // 7. Dispatch tool calls (parallel + serial batching)
    let results = self.dispatcher.dispatch_batch(calls, ctx).await;

    // 8. Format results as messages
    result_msg::append_results(&mut messages, self.translator.render_results(&results));

    // 9. Prune context if needed
    prune::prune_if_needed(&mut messages, self.context_token_limit);

    iterations += 1;
}
```

---

## 6. Submodules

| Submodule | Module | Purpose |
|-----------|--------|---------|
| `max_iter` | `tool_loop/max_iter.rs` | Iteration cap enforcement |
| `prune` | `tool_loop/prune.rs` | Context-growth pruning |
| `result_msg` | `tool_loop/result_msg.rs` | Tool-result message construction |
| `checkpoint` | `tool_loop/checkpoint.rs` | Resumable state snapshots |

---

## 7. Checkpoint and Resume

When the loop stops for any reason other than `Stop`, it produces a `Checkpoint`:

```rust
pub struct Checkpoint {
    pub iterations: usize,
    pub tool_calls: Vec<ToolCall>,
    pub messages: Vec<serde_json::Value>,
}
```

The checkpoint captures full conversation state:

```rust
// Resume from where we left off
let output = tool_loop.resume(checkpoint, &tools, &ctx).await;
```

This is critical for long-running tasks that hit the iteration cap or experience
transient backend errors. The conversation does not need to restart from scratch.

---

## 8. Context Pruning

The `prune` submodule implements context-growth guards:

```rust
pub fn prune_if_needed(messages: &mut Vec<Value>, token_limit: usize);
```

Pruning strategy:

1. Estimate tokens from message byte length (conservative 4 bytes/token)
2. If total exceeds limit, begin dropping messages
3. **Always preserve:** system prompt, first user message
4. **Preserve:** most recent N messages (tail window)
5. **Preserve:** tool results with errors (diagnostic value)
6. **Drop first:** oldest tool results in the middle

The strategy is conservative: the model always has its original instructions
and the most recent context. Aggressive pruning does not lose data permanently
because checkpoints save full state.

---

## 9. The 7-Step ToolDispatcher Pipeline

The `ToolDispatcher` at `crates/roko-agent/src/dispatcher/mod.rs` processes
every tool call:

```
1. VALIDATE   -- identity + args against JSON schema from registry
2. AUTHORIZE  -- profile/task filters and role capabilities
3. SAFETY     -- hooks, policy, durable immune controls
4. EXECUTE    -- handler under timeout/cancellation, panic-catching
5. BOUND      -- recursively scrub, recover, re-bound results
6. SCREEN     -- finalized result through fixed immune Graph
7. FINALIZE   -- emit one sanitized terminal audit signal
```

### Batch dispatch

`dispatch_batch` groups calls by `ToolConcurrency`:

- **Parallel** tools (read_file, grep, glob) run through bounded unordered
  stream via `join_all`
- **Serial** tools (bash, write_file) run sequentially to preserve ordering
  and avoid write-write races

### Dispatcher limits

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_TOOL_CALLS_PER_BATCH` | 16 | Per-turn call cap |
| `MAX_TOOL_CALL_INGRESS_BYTES` | 256 KB | Per-call size limit |
| `MAX_TOOL_CALL_FRAME_BYTES` | 1 MB | Per-frame size limit |
| `MAX_TOOL_BATCH_RESULT_BYTES` | 8 MB | Aggregate result cap |

### Dispatcher submodules

| Module | Purpose |
|--------|---------|
| `alert` | Alert emission |
| `cancel` | Cancellation primitives |
| `dedup_cache` | Dispatch-level dedup for idempotent dispatch |
| `emit_metric` | Metric emission |
| `hook_chain` | Pre/post execution hook chains |
| `parallel` | Concurrency partitioning |
| `production_safety_chain` | Production safety hook chain |
| `result_cache` | Explicit cache primitives (dispatcher does NOT cache internally) |
| `timeout` | Timeout enforcement |
| `tool_selector` | Tool selection logic |
| `truncate` | Result truncation/bounding |
| `validate` | Input validation |

---

## 10. SafetyLayer Integration

The `SafetyLayer` at `crates/roko-agent/src/safety/mod.rs` composes six
policy families, applied at step 3 of the dispatcher:

```rust
pub struct SafetyLayer {
    pub bash_policy: BashPolicy,       // Command allowlist/denylist
    pub git_policy: GitPolicy,         // Branch protection
    pub network_policy: NetworkPolicy, // Outbound destination allowlist
    pub path_policy: PathPolicy,       // Worktree escape prevention
    pub scrub_policy: ScrubPolicy,     // Secret scrubbing from outputs
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub role: String,
}
```

Pre-execution: `check_pre_execution` applies policies based on tool name.
Post-execution: `scrub_output` removes API keys, tokens, and secrets from
tool output before it enters conversation history.

---

## 11. Reasoning Pattern Hierarchy

The ToolLoop implements the basic ReAct pattern. Research identifies a hierarchy
of reasoning patterns, each building on the previous:

| Pattern | Quality | Cost | Best for | Source |
|---------|---------|------|----------|--------|
| **Direct** | Low | 1 call | Simple classification | -- |
| **ReAct** | Medium | N calls | Standard tool use (Roko) | Yao et al., 2023 |
| **Reflexion** | High | 2N calls | Tasks with gate feedback | Shinn et al., 2023, NeurIPS |
| **Tree-of-Thought** | Higher | K x N | Plan generation, exploration | Yao et al., 2023, NeurIPS |
| **MCTS/LATS** | Highest | K^2 x N | Hard debugging, architecture | Zhou et al., 2024, ICML |

Roko's gate failure replan (`build_gate_failure_plan_revision` in the runner)
implements a form of Reflexion: gate results are converted to verbal feedback
and injected into the next agent dispatch.

---

## 12. Test Coverage

The ToolLoop has comprehensive tests covering all stop conditions:

- `zero_tool_calls_returns_immediately` -- No tools -> immediate final answer
- `single_tool_call_runs_to_completion` -- One tool call -> dispatch -> final
- `max_iterations_returns_max_iterations` -- Hits cap -> checkpoint
- `cancellation_halts_loop` -- Cancel token -> stops between turns
- `backend_error_returns_backend_error` -- LLM error -> checkpoint
- `parallel_tool_calls_dispatched_in_one_batch` -- Multiple calls -> parallel
- `context_prune_drops_oldest_results_after_threshold` -- Pruning works
- `tool_call_ids_flow_through_to_result_messages` -- ID propagation
- `resume_continues_from_checkpoint` -- Checkpoint -> resume -> continues

---

## 13. Citations

1. Yao, S. et al. (2023). "ReAct: Synergizing Reasoning and Acting in Language Models." ICLR 2023. arXiv:2210.03629.
2. Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal
   Reinforcement Learning." NeurIPS 2023. arXiv:2303.11366.
3. Zhou, A. et al. (2024). "Language Agent Tree Search Unifies Reasoning,
   Acting, and Planning." ICML 2024. arXiv:2310.04406.
4. Yao, S. et al. (2023). "Tree of Thoughts: Deliberate Problem Solving with
   Large Language Models." NeurIPS 2023. arXiv:2305.10601.
5. `crates/roko-agent/src/tool_loop/mod.rs` -- ToolLoop, LlmBackend,
   Checkpoint, ToolLoopOutput.
6. `crates/roko-agent/src/dispatcher/mod.rs` -- ToolDispatcher 7-step pipeline.
7. `crates/roko-agent/src/safety/mod.rs` -- SafetyLayer, 6 policy families.
