# Safety & Agent System: Fix Plan

Ordered by severity. Each item references the corresponding issue in ISSUES.md.

---

## Phase 1: Critical + High (fail-closed, eliminate silent bypasses)

### P1-1: Switch `contract_for_role()` to `RestrictedFallback` (ISS-1)

**File:** `crates/roko-agent/src/safety/mod.rs`

Change `contract_for_role()` to use `AgentContract::load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)` instead of `load_for_role(...).unwrap_or_else(permissive)`.

`RestrictedFallback` already exists and produces a deny-all contract with `allowed_tools: Some(vec![])`. The warning log is already included. This is a one-line change.

```rust
fn contract_for_role(&self, role: &str) -> AgentContract {
    let mut contract = AgentContract::load_for_role_with_mode(
        role,
        ContractLoadMode::RestrictedFallback,
    ).unwrap_or_else(|_| AgentContract::restricted(role)); // Strict mode never errors; RestrictedFallback is infallible
    ...
}
```

Note: `load_for_role_with_mode` with `RestrictedFallback` is already infallible — it returns `Ok(restricted)` on any load error. The outer `unwrap_or_else` becomes unreachable but harmless.

### P1-2: Change `dangerously_skip_permissions` default to `false` (ISS-2)

**File:** `crates/roko-agent/src/claude_cli_agent.rs:128`

Change the default from `true` to `false`. Callers that need to skip permissions must explicitly opt in. Add a `#[doc]` comment explaining the security implications.

### P1-3: Enforce safety layer attachment (ISS-3)

**File:** `crates/roko-agent/src/dispatcher/mod.rs`

Option A (preferred): Change `safety: Option<SafetyLayer>` to `safety: SafetyLayer`. Require safety at construction. This is a breaking change but eliminates the silent bypass.

Option B (lower friction): Add a `#[must_use]` warning or assertion in `dispatch()` when `safety` is `None`.

---

## Phase 2: Medium (close remaining gaps)

### P2-1: Wire cumulative spend tracking (ISS-4)

**File:** `crates/roko-agent/src/safety/contract.rs`

`GovernanceRule::MaxCostPerTurn` needs a per-turn accumulator in `ToolContext`. Thread cumulative cost through `ctx.external_actions` metadata or add a dedicated `ctx.turn_spend_usd: Arc<AtomicF64>`. Update `check()` to sum all actions and compare against the limit.

Resolves TODO(UX26).

### P2-2: Default safety budget to non-None (ISS-5)

**File:** `crates/roko-agent/src/safety/mod.rs`

`SafetyLayer::with_defaults()` should instantiate a `SafetyBudgetTracker` with permissive limits (e.g. `BudgetDimension::TotalCostUsd(1000.0)`) rather than `None`. Callers can override with tighter limits.

### P2-3: Wire orchestrator-level recovery actions (ISS-6)

**File:** `crates/roko-cli/src/orchestrate.rs`

After `post_dispatch_check()` returns violations of type `ContractViolation`, look up `self.safety_layer.contract.applicable_recovery()` against a synthetic `ToolResult::Err`. Dispatch the recovery action (at minimum: log for `Alert`, abort task for `Abort`, retry for `Retry`).

### P2-4: Add per-task rate limit scope (ISS-7)

**File:** `crates/roko-agent/src/safety/rate_limit.rs` and `safety/mod.rs`

Add a `task_id: Option<String>` to `RateLimitKey`. Reset counters when a new task starts by calling a `reset_for_task(task_id)` method. Alternatively, scope `RateLimiter` to task lifetime rather than session.

### P2-5: Complete post-execution checks (ISS-8)

**File:** `crates/roko-agent/src/safety/mod.rs`

Extend `post_dispatch_check()` to validate:
- Tool call count from `changed_files` or an explicit `tool_call_count` argument vs `MaxToolCallsPerTurn`
- Cumulative spend vs `MaxCostPerTurn` (once ISS-4 is resolved)
- Trailing failure count vs `MaxConsecutiveFailures`

### P2-6: Deduplicate contract check in dispatcher (ISS-9)

**File:** `crates/roko-agent/src/dispatcher/mod.rs`

Remove the explicit `safety.check_contract()` call (line ~346). `check_pre_execution()` already calls `self.contract.check_pre_execution()` at the end. The separate `check_contract()` call is redundant.

---

## Phase 3: Low (hygiene)

### P3-1: Fix contract file format mismatch (ISS-11)

Rename `roko-agent/src/safety/contracts/*.yaml` to `*.json`, or switch the loader to use a YAML parser. JSON-in-YAML-extension is confusing. Since the files are already valid JSON and the loader uses `serde_json`, renaming to `.json` is the lowest-effort fix. Update `contract_asset_path()` in `contract.rs:505`.

### P3-2: Warn on missing MCP config (ISS-10)

**File:** `crates/roko-agent/src/process/mcp.rs` (or wherever `find_mcp_config()` lives)

Emit `tracing::warn!` when `mcp_config` is `Some(path)` but the path does not exist, or when discovery finds no config.

---

## Sources

- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contract.rs`
- `crates/roko-agent/src/dispatcher/mod.rs`
- `crates/roko-agent/src/claude_cli_agent.rs`
- `crates/roko-cli/src/orchestrate.rs`
