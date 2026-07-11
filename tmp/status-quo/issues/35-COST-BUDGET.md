# Cost and Budget Tracking Issues

## Critical

### Budget enforcement is log-only — run continues
- `event_loop.rs:1072-1087`: Turn exceeds `max_turn_usd` → `warn!` → continues normally.
- `orchestrate.rs:16620-16624`: `BlockNewSessions` → `tracing::error!` → does NOT block dispatches.
- `orchestrate.rs:16647-16651`: `RouteToCheaper` → `tracing::warn!` → does NOT switch model.

### Token counts zero for all non-Claude-CLI providers
- Gemini (`native.rs:511`), Ollama (`agent.rs:184,207`), Claude API (`claude_agent.rs:128,363`), Cursor (`cursor_agent.rs:103,425`): All return `cost_usd: None`.
- Cost comes from `CostTable::calculate()` which returns `0.0` when tokens are zero → turns recorded as free.

### CostsDb in-memory only in runner-v2
- `event_loop.rs:990`: Fresh `CostsDb::new()` at run start. Never serialized to disk. All records lost on exit.
- `CostsLog` (file-backed) exists but never called from runner-v2.

## High

### Duplicate BudgetGuardrail types with independent state
- `roko-learn/budget.rs:8` and `roko-agent/task_runner.rs:198`: Same fields, same logic, different instances.
- `orchestrate.rs` uses both. Cumulative spend tracked by one is invisible to the other.

### CostTable falls back to Sonnet rates silently
- `cost_table.rs:73-77`: Unknown model + tokens > 0 → `SONNET_FALLBACK` ($3/$15 per M).
- Haiku overstated 10x. Opus understated 5x. No warning logged.

### No cost attribution by role in runner-v2
- `event_subscriber.rs:151-164`: `role`, `plan_id`, `complexity_band`, `session_id` all empty strings.
- `CostsDb::summary_by_role()`, `by_plan()`, `by_session()` all return empty/single-bucket.

## Medium

### Resume resets cost to 0.0
- `resume.rs:321`: `total_cost_usd: 0.0` initial. Restored from snapshot at `event_loop.rs:3863` IF snapshot write succeeded. Failed snapshot → budget appears empty.

### Duplicate CostTable types within same crate
- `cost_table.rs:37`: Used for actual computation.
- `costs_db.rs:164`: Richer (tiered pricing, per-request) but never used in any real computation.

### No combined budget scope enforcement
- `orchestrate.rs:16594-16601`: Fresh `BudgetGuardrail::new()` per task. Checks each scope in isolation. No combined task+session check.
