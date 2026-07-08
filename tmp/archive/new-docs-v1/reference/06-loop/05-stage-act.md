# ACT — Stage 5 of the Cognitive Loop

> Execute the selected capability against the composed context and collect the raw output.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [ComposedContext](04-stage-compose.md),
[Policy operator](../05-operators/policy.md)
**Used by**: [VERIFY](06-stage-verify.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

ACT is where the agent does something. It passes the `ComposedContext` to the selected
target — a model API call, a tool invocation, or a sub-agent dispatch — and collects
the raw `ActOutput`. ACT applies pre-execution policy checks (rate limits, cost caps)
but does not evaluate quality; that is VERIFY's job.

---

## The Idea

ACT is the only stage that crosses the agent boundary into the external world. Every
other stage is pure data transformation within the Roko runtime. ACT is where:

- Model tokens are consumed (and costs accrue).
- External tools are called (and side effects happen).
- Sub-agents are dispatched (and latency multiplies).

This privileged position has two consequences. First, ACT is where the budget
controller must enforce hard limits — a model call that goes over budget cannot be
un-called. Second, ACT is the only stage that may block on I/O; all other stages are
expected to be CPU-bound and fast.

Roko handles both consequences explicitly: the Policy operator gates entry to ACT, and
ACT runs with a configurable timeout enforced by the Harness.

---

## Specification

```rust
// source: crates/roko-agent/src/loop/act.rs
pub struct ActOutput {
    pub content:     ActContent,
    pub token_cost:  TokenCost,
    pub wall_time_ms: u64,
    pub metadata:    HashMap<String, serde_json::Value>,
}

pub enum ActContent {
    Text(String),
    ToolResult(serde_json::Value),
    SubAgentFuture(AgentFuture),  // for async sub-agent dispatches
}

pub struct TokenCost {
    pub prompt_tokens:     u32,
    pub completion_tokens: u32,
    pub total_cost_usd:    f64,
}

pub trait ActStage: Send + Sync {
    async fn act(
        &self,
        context:  &ComposedContext,
        policy:   &dyn Policy,
        budget:   &TickBudget,
    ) -> Result<ActOutput, ActError>;
}
```

---

## Pre-Execution Policy Check

Before calling the external target, ACT invokes `policy.pre_act(context, budget)`.
The policy may:

- **Allow** — proceed normally.
- **Throttle** — add a delay before proceeding (rate-limiting).
- **Modify** — strip or redact content from the context before calling the model.
- **Block** — return `ActError::PolicyBlock` without calling the target.

The pre-act check is synchronous and must complete within 1 ms. Policies that require
slow lookups (e.g., checking a blocklist in a remote service) should cache their
results.

---

## Execution and Timeout

ACT calls the target capability and waits for a response within `TickBudget.act_timeout`.
Default timeouts:

| Target type | Default timeout | Max timeout |
|---|---|---|
| Model API | 30 s | 120 s |
| Tool (local) | 5 s | 30 s |
| Tool (remote) | 15 s | 60 s |
| Sub-agent (async) | Returns immediately; future resolved in next tick | — |

If the target does not respond within the timeout, ACT returns `ActError::Timeout`.
The tick then proceeds to VERIFY, which will fail the result, causing PERSIST to
record a `act.timeout` Engram.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `ActError::PolicyBlock` | Policy blocked the execution | Record `act.blocked` Engram; tick ends cleanly |
| `ActError::Timeout` | Target did not respond in time | Proceed to VERIFY with null output; VERIFY fails gracefully |
| `ActError::ModelError` | API returned 4xx/5xx | Retry once with exponential backoff; if still fails, abort tick |
| `ActError::CostExceeded` | Estimated cost > budget | Policy blocks before call is made |
| `ActError::ToolError` | Tool returned an error payload | Treat as failed output; VERIFY decides if it is recoverable |

---

## Performance

| Metric | Notes |
|---|---|
| Pre-policy check | < 1 ms (must not block on I/O) |
| Model call latency | External — typically 500 ms – 30 s |
| Tool call latency | External — typically 50 ms – 5 s |
| Sub-agent dispatch (async) | < 1 ms (fire-and-forget; result in later tick) |

ACT is the dominant source of tick latency in almost all real-world runs. The other
seven stages combined typically account for < 50 ms; a model call is 500 ms–30 s.
This makes model selection and routing the primary levers for latency reduction —
not loop optimization.

---

## Examples

### 1. Standard model call

COMPOSE produced a 3 000-token context for `model=gpt-4o`. Policy allows. ACT calls
the OpenAI API, receives a 400-token completion in 1.2 s. `ActOutput.token_cost` is
recorded for budget accounting.

### 2. Policy block

An agent in a sandboxed environment is configured with a policy that blocks all
`web_search` tool calls. ACT receives a context for `tool=web_search`, calls
`policy.pre_act()`, gets `PolicyDecision::Block`. ACT returns `PolicyBlock` without
calling the tool. The tick ends with a `act.blocked` Engram.

### 3. Sub-agent dispatch

A planning agent dispatches a research sub-agent to gather information asynchronously.
ACT fires the sub-agent, receives an `AgentFuture`, and returns immediately. The parent
tick continues; the sub-agent's result arrives as a Pulse in a future tick.

---

## See also

- [Policy operator](../05-operators/policy.md) — pre-execution and post-execution policy
- [COMPOSE](04-stage-compose.md) — prepares the context consumed here
- [VERIFY](06-stage-verify.md) — evaluates the output produced here
- [Failure Modes](13-failure-modes.md) — cross-stage failure taxonomy
- [Performance](14-performance.md) — end-to-end latency breakdown
