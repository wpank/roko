# 27.03 -- Budget Enforcement

> Depth file for [27-ACP.md](../../27-ACP.md).

---

## Overview

Each ACP session tracks accumulated USD cost and enforces a configurable budget
cap. When the budget is exhausted, the session rejects further prompts with
error code `-32002` (`SESSION_BUDGET_EXCEEDED`).

## Session Cost State

Every ACP session maintains running cost counters:

```rust
pub struct SessionBudgetStatus {
    pub total_cost_usd: f64,
    pub budget_limit_usd: Option<f64>,
    pub budget_remaining_usd: Option<f64>,
    pub turns_used: u32,
    pub input_tokens_total: u64,
    pub output_tokens_total: u64,
}
```

Cost is computed from provider usage reports after each `session/prompt` call.
The `BudgetGuardrail` from `roko-learn` provides the enforcement boundary:

```rust
pub struct BudgetGuardrail {
    max_usd: f64,
    spent_usd: f64,
}

impl BudgetGuardrail {
    pub fn check(&self, estimated_cost: f64) -> BudgetAction {
        if self.spent_usd + estimated_cost > self.max_usd {
            BudgetAction::Deny
        } else {
            BudgetAction::Allow
        }
    }
}
```

## Budget Configuration

Budget limits are set at the workspace level in `roko.toml`:

```toml
[agent]
max_cost_usd = 5.0  # Per-session cost cap
```

They can also be set per-session via `session/config/update`:

```json
{
  "sessionId": "abc-123",
  "optionId": "budget",
  "newValue": "10.0"
}
```

## Enforcement Points

### Pre-Prompt Check

Before dispatching a prompt to the LLM provider, the session checks the
budget guardrail:

1. Estimate the cost of the upcoming turn (based on model pricing and
   estimated token count)
2. Call `guardrail.check(estimated_cost)`
3. If `BudgetAction::Deny`, return `SESSION_BUDGET_EXCEEDED` error

### Post-Prompt Accounting

After a successful prompt dispatch:

1. Extract usage from the provider response (input/output tokens)
2. Compute cost using the model's per-token pricing
3. Update `SessionBudgetStatus` totals
4. Persist the session state to disk

Even when a post-dispatch transport or task error occurs, cost is still
recorded because the provider call may have already been billed.

```rust
// Persist even when a post-dispatch transport/task error occurs: a
// completed provider call may already have accrued billable cost.
sessions.persist_session(&session_id_for_persist);
```

## Provider Health Integration

Budget enforcement interacts with the provider health system. Providers that
consistently exhaust budgets or produce high-cost failures are factored into
routing decisions by the `ProviderHealthRegistry`:

```rust
pub struct ProviderHealthRegistry {
    // Provider health state includes cost efficiency metrics
}

impl ProviderHealthChecker for ProviderHealthRegistry {
    fn is_healthy(&self, provider: &str) -> bool;
}
```

Unhealthy providers are filtered during learned routing, which indirectly
helps budget preservation by avoiding wasteful retries on failing providers.

## Budget Status in Responses

Budget status is included in several ACP responses:

### `session/new` Response

```json
{
  "sessionId": "abc-123",
  "budget": {
    "totalCostUsd": 0.0,
    "budgetLimitUsd": 5.0,
    "budgetRemainingUsd": 5.0,
    "turnsUsed": 0,
    "inputTokensTotal": 0,
    "outputTokensTotal": 0
  }
}
```

### `session/prompt` Response

Each prompt response includes updated budget status so the IDE can display
remaining budget in the UI.

### Config Options

Budget appears as a configurable option in the session config:

```rust
ConfigOption {
    id: "budget",
    label: "Session Budget (USD)",
    option_type: ConfigOptionType::Number,
    current_value: ConfigOptionValue::Number(5.0),
}
```

## Session Persistence

Budget state is persisted as part of the session state to survive process
restarts:

```json
{
  "sessionId": "abc-123",
  "provider": "anthropic",
  "totalCostUsd": 2.35,
  "turnsUsed": 8,
  "inputTokensTotal": 15000,
  "outputTokensTotal": 4200,
  "budgetLimitUsd": 5.0,
  "history": [...]
}
```

Sessions are persisted to `.roko/sessions/` as individual JSON files. The
`SessionManager` handles GC of sessions older than 7 days.

## Rate Limiting Integration

The ACP server also integrates with the provider rate limiter:

```rust
pub struct ProviderRateLimiter {
    // Per-provider rate limiting state
}
```

This prevents a single session from overwhelming a provider's rate limits,
which would cause 429 errors and wasted budget on retried requests.

## Source

- `crates/roko-acp/src/session.rs` -- Session state and budget tracking
- `crates/roko-acp/src/runner.rs` -- Pipeline runner with cost accounting
- `crates/roko-learn/src/budget.rs` -- BudgetGuardrail and BudgetAction
- `crates/roko-learn/src/provider_health.rs` -- ProviderHealthRegistry
