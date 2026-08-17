# Runner 09 — Safety Layer Wiring

> **Give this entire file to a fresh agent.**

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko`. Goal: wire `SafetyLayer` into the unified `EffectDriver` so every model call is bracketed by pre/post safety checks. Make `dangerously_skip_permissions` come from contracts, not hardcoded. Enforce cumulative per-turn cost.

**Read first:**

1. `tmp/workflow/implementation-plans/09-safety-layer-wiring.md`
2. `crates/roko-agent/src/safety/mod.rs` — `SafetyLayer`, `pre_dispatch_check`, `post_dispatch_check`
3. `crates/roko-agent/src/safety/contract.rs` — `AgentContract`
4. `crates/roko-agent/src/safety/contracts/` — 8 YAML contracts
5. `crates/roko-runtime/src/effect_driver.rs` — where to wire safety

---

## Work Items

### 1. Add `safety` to `EffectServices`

```rust
pub safety: Arc<SafetyLayer>,
```

### 2. Bracket every agent spawn

In `EffectDriver::spawn_for_role` (or equivalent):

```rust
// PRE-DISPATCH
self.services.safety.pre_dispatch_check(plan_id, task, role, exec_dir)?;

// SCRUB assembled prompt
let scrubbed = self.services.safety.scrub(&assembled.system);

// ... call model ...

// POST-DISPATCH
self.services.safety.post_dispatch_check(plan_id, task, role, &response.content, &changed_files);

// RECOVERY CHECK
if let Some(recovery) = self.services.safety.check_agent_recovery(role, &response, &task) {
    return self.apply_recovery(recovery).await;
}
```

### 3. Contract-based `dangerously_skip_permissions`

Add `dangerously_skip_permissions: bool` to `AgentContract` YAML schema. Set `true` only for `implementer.yaml`. All others `false`.

Replace all hardcoded `dangerously_skip_permissions: true` in the codebase with `contract.dangerously_skip_permissions`.

Verify: `rg 'dangerously_skip_permissions' crates/ --type rust | grep -v 'safety/contract' | grep -v test` returns 0.

### 4. Cumulative per-turn cost

Create `crates/roko-agent/src/safety/budget.rs`:

```rust
pub struct PerTurnSpend { by_role: HashMap<String, f64> }
impl SafetyLayer {
    pub fn record_call_cost(&mut self, role: &str, cost: f64, contract: &AgentContract) -> SafetyResult;
    pub fn reset_turn(&mut self);
}
```

Call `record_call_cost` after every successful model call in `EffectDriver`. `CostExceeded` → `EffectOutcome::Failed`.

### 5. MCP loud failure

In `crates/roko-agent/src/provider/mod.rs`, find `find_mcp_config`. If missing and role expects MCP tools → log `warn!` (Optional) or error (Required).

### 6. Agent recovery

Add `check_agent_recovery(role, response, task) -> Option<RecoveryAction>` to `SafetyLayer`. 3 consecutive failures → `Alert`. Tokens exceed contract max → `Downgrade`.

---

## Verification

```bash
rg 'pre_dispatch_check' crates/roko-runtime/src/effect_driver.rs
# returns 1+

rg 'dangerously_skip_permissions' crates/ --type rust | grep -v 'safety/contract' | grep -v test
# returns 0

rg 'record_call_cost' crates/roko-agent/src/safety/
# returns 1+

cargo test --workspace
```
