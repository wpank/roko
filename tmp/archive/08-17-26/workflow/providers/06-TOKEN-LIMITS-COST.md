# 06 — Token Limits, Budgets & Cost Tracking

## The Problem

Mori has role-specific token budgets with model multipliers and accurate cost tracking
from Claude's `total_cost_usd`. Roko has inconsistent hardcoded max_tokens values and
broken cost tracking (always shows $0).

---

## How Mori Does It

### Per-Role Budget (in USD, not tokens)
**File**: `connection.rs:615-640`

| Role | Budget | Notes |
|------|--------|-------|
| Implementer | $1.50 | Highest — does the most work |
| Strategist, Researcher | $0.75 | |
| Conductor | $0.50 | Read-only orchestrator |
| Auditor, QuickReviewer | $0.50 | |
| Scribe, Critic | $0.40 | |
| AutoFixer | $0.75 | |
| Others | $0.50 | Default |

### Model Multipliers
| Model tier | Multiplier | Effective for Implementer |
|-----------|-----------|--------------------------|
| Opus | 2.0x | $3.00 |
| Sonnet | 1.0x | $1.50 |
| Haiku | 0.6x (min $0.35) | $0.90 |

### Override
`MORI_CLAUDE_MAX_BUDGET_USD` env var overrides all budgets.

### Cost Tracking
```rust
// From Result event:
ClaudeResultEvent {
    total_cost_usd: Option<f64>,  // Claude gateway reports actual cost
    usage: Option<ClaudeUsage>,    // Token counts
}

// Delta computation:
let delta = (reported_cost - last_reported_cost).max(0.0);
cumulative_cost_usd += delta;
```

### Token Usage
```rust
AgentEvent::TokenUsage {
    input_tokens: u64,
    output_tokens: u64,
    context_window: Option<u64>,
    cost_usd: Option<f64>,  // From gateway or Result event
}
```

---

## How Roko Does It

### Hardcoded Max Tokens (inconsistent across paths)

| File | Line | Value | Path |
|------|------|-------|------|
| `dispatch_direct.rs` | 212 | **8192** | Anthropic API direct |
| `dispatch_direct.rs` | 296 | **8192** | OpenAI-compat direct |
| `anthropic_api/tool_loop.rs` | 242 | **4096** | roko-agent Anthropic adapter |
| `roko-core/config/agent.rs` | 259 | **4096** | Default data_llm config |
| `gateway.rs` | 991 | **1024** | HTTP gateway route |
| `demo/scenarios/llm.rs` | 257 | **512** | Demo scenario |
| `neuro/distiller.rs` | 26 | **2048** | Knowledge distillation |

The same user doing `roko chat` (8192) vs `roko run` through the agent layer (4096)
gets different token limits for the same model.

### Cost Tracking (broken)
```rust
// chat_inline.rs:1100
let cost = cost_from_result(&session.cost_table, &result);
// But cost_from_result calls CostTable::calculate which returns 0.0
// because the cost table entries for most models are missing/zero

// unified.rs:95-103
// No cost displayed at all in one-shot mode
```

### Token Approximation
```rust
// chat_inline.rs:3250-3258
let input_tokens = if resp_input > 0 {
    resp_input
} else {
    (message.len() as u64) / 4  // Guess: 4 chars per token
};
let output_tokens = if resp_output > 0 {
    resp_output
} else {
    (clean.len() as u64) / 4  // Guess
};
```

When the backend doesn't report tokens, roko guesses from string length.
This is wildly inaccurate.

---

## What's Wrong

### 1. Max tokens too low
4096 tokens is too small for implementation tasks. Claude Sonnet/Opus can generate
much longer responses. Mori doesn't limit output tokens at all — it uses USD budgets
instead, letting Claude decide response length.

### 2. Inconsistent limits
Three different default values (4096, 8192, 1024) depending on which code path is hit.

### 3. No role-based budgets
Roko has `budget.max_task_usd = 1.0` in global config but this isn't applied
per-turn. Each turn runs with whatever hardcoded max_tokens the path uses.

### 4. Cost always $0
`CostTable` exists but entries for most models are missing or zero.
The `total_cost_usd` from Claude CLI Result events is captured (now, after fix)
but not connected to the cost display.

### 5. No cache token tracking
Claude reports `cache_creation_input_tokens` and `cache_read_input_tokens`.
Mori captures these. Roko ignores them entirely.

---

## What Needs to Change

1. **Increase default max_tokens** to 16384 or remove the limit (use USD budgets)
2. **Make max_tokens configurable** in `roko.toml` per-model or per-role
3. **Fix cost tracking** — use `total_cost_usd` from Result events when available
4. **Populate CostTable** with known model pricing (or compute from token counts)
5. **Track cache tokens** for accurate cost computation
6. **Apply per-role budgets** matching mori's structure
