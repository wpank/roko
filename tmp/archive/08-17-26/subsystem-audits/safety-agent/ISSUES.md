# Safety & Agent System: Issues

Verified against source as of 2026-04-28.

---

## CRITICAL

### ISS-1: Contract fail-open in `contract_for_role()`

**File:** `crates/roko-agent/src/safety/mod.rs:864-871`

`SafetyLayer::with_role()` calls `contract_for_role()` which uses `AgentContract::permissive` as fallback:

```rust
fn contract_for_role(&self, role: &str) -> AgentContract {
    let mut contract = AgentContract::load_for_role(role).unwrap_or_else(|err| {
        tracing::warn!(%role, %err, "no contract for role; using permissive default");
        AgentContract::permissive(role.to_string())  // zero restrictions
    });
    ...
}
```

`AgentContract::permissive` has empty `invariants`, `governance`, `recovery`, and `allowed_tools: None` — all restrictions absent.

The safer `load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)` — which returns a deny-all contract — exists in `contract.rs:153` but is not used in this path.

**Impact:** Typo in role name or missing JSON asset → zero contract enforcement.

---

## HIGH

### ISS-2: `dangerously_skip_permissions` defaults to `true`

**File:** `crates/roko-agent/src/claude_cli_agent.rs:128`

```rust
dangerously_skip_permissions: true,
```

The `ClaudeCliAgent::new()` constructor enables this flag by default, passing `--dangerously-skip-permissions` to the Claude CLI subprocess. This bypasses Claude's own permission prompts.

Exposed as `AgentOptions::dangerously_skip_permissions` in `provider/mod.rs:437` and threaded into the adapter in `provider/claude_cli.rs:54`.

**Impact:** All Claude CLI agent invocations run without interactive permission guards.

### ISS-3: `safety: Option<SafetyLayer>` in dispatcher

**File:** `crates/roko-agent/src/dispatcher/mod.rs:89`

`ToolDispatcher::new()` sets `safety: None`. Callers must explicitly call `.with_safety(layer)`. Any code path that constructs a dispatcher without attaching a layer gets zero safety enforcement silently.

---

## MEDIUM

### ISS-4: Cumulative per-turn spend not enforced

**File:** `crates/roko-agent/src/safety/contract.rs:437-449`

`GovernanceRule::MaxCostPerTurn` checks only the single-call `estimated_cost_usd` argument from the tool call arguments. Cumulative spend across all tool calls in a turn is not tracked.

TODO(UX26) comment in source marks this gap.

### ISS-5: Safety budget not instantiated by default

**File:** `crates/roko-agent/src/safety/mod.rs:247`

`SafetyLayer::with_defaults()` sets `safety_budget: None`. The `SafetyBudgetTracker` (adaptive-risk budget with `BetaDistribution`) is built and wired but requires explicit opt-in.

### ISS-6: Recovery not invoked at orchestrator level

Recovery actions (`Retry`, `Downgrade`, `Abort`, `Alert`) are invoked per-tool-call via `SafetyLayer::check_recovery()` in the dispatcher (`dispatcher/mod.rs:456-462`). They are **not** evaluated at the orchestrator level for task-level failures. `applicable_recovery()` is not consulted from `orchestrate.rs`.

### ISS-7: Rate limiter has no per-task reset

The `RateLimiter` in `SafetyLayer` is scoped per `(role, tool)` globally — not per task or per turn. Rate limits persist across tasks within a session. No mechanism to reset counters when a new task starts.

### ISS-8: Post-execution checks incomplete

`SafetyLayer::post_dispatch_check()` in `safety/mod.rs:696` checks only:
1. Secret leaks in agent output (scrub)
2. Path escapes in changed file paths
3. ForbiddenTools write check

Missing checks:
- Tool call count vs `MaxToolCallsPerTurn`
- Cumulative spend vs `MaxCostPerTurn`
- Consecutive failure count vs `MaxConsecutiveFailures`

### ISS-9: Duplicate contract check in dispatcher

`ToolDispatcher::dispatch()` calls `safety.check_pre_execution()` at line ~332, which internally calls `self.contract.check_pre_execution()` at the end. The dispatcher then also calls `safety.check_contract()` at line ~346, which is a thin wrapper around the same `contract.check_pre_execution()` call. Contract invariants and governance rules are evaluated twice per dispatch.

---

## LOW

### ISS-10: MCP config silent failure

Missing `.mcp.json` (or the path specified in `AgentOptions.mcp_config`) causes agents to run without MCP tools, with no warning emitted. `find_mcp_config()` returns `None` silently.

### ISS-11: Contract files are JSON, not YAML

Contract files in `roko-agent/src/safety/contracts/` have `.yaml` extensions but are parsed via `serde_json::from_str()` (see `contract.rs:129`). The files contain JSON, not YAML. This is misleading for contributors adding new contracts — standard YAML features (anchors, comments) will fail to parse.

---

## Sources

- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contract.rs`
- `crates/roko-agent/src/dispatcher/mod.rs`
- `crates/roko-agent/src/claude_cli_agent.rs`
- `crates/roko-agent/src/provider/mod.rs`
