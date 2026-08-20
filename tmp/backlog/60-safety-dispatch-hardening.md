# 60 — Safety Dispatch Hardening

**Priority**: P1 — security: three structural weaknesses in the fail-closed safety invariant
**Size**: M (3 days)
**Crates**: `crates/roko-agent/src/safety/`, `crates/roko-agent/src/provider/`, `crates/roko-cli/src/runner/`
**Depends on**: None

---

## Background

The safety layer in `roko-agent` enforces tool-level and dispatch-level security policies for every agent execution path. Its design intent is fail-closed: an unknown role receives zero capabilities, and the safety layer cannot be absent during production dispatch.

Three structural weaknesses undermine this invariant today. First, a role that has only a budget override in `roko.toml` (no explicit `tools` key) silently receives unrestricted tool access — the opposite of the intended deny-all behavior. Second, `RunConfig` (the runner's dispatch configuration) holds `safety_layer: Option<SafetyLayer>` and its `Default` implementation sets it to `None`, meaning any struct literal that uses `..Default::default()` exercises real dispatch code with zero safety enforcement. Third, the Hermes and OpenClaw external provider adapters construct agents with no `SafetyLayer` at all, so their outputs bypass roko's secret-scrubbing and taint-checking boundary.

## Current State

1. `SafetyLayer::contract_for_role` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs:1250-1304`. When `load_for_role_with_mode` returns a restricted fallback (deny-all: `allowed_tools = Some(vec![])`), lines 1282-1289 check:
   ```rust
   if contract.allowed_tools.as_ref().is_some_and(|t| t.is_empty())
       && (self.role_tools.contains_key(role) || self.role_overrides.contains_key(role))
   {
       contract.allowed_tools = None;  // ← clears deny-all
   }
   ```
   `self.role_overrides` is populated from `[agent.roles]` entries including entries that have only `budget` fields and no `tools` key. The disjunction `|| self.role_overrides.contains_key(role)` means a role with `[agent.roles.custom-role]\nbudget.max_cost_usd_per_turn = 1.0` and no `tools` list clears the deny-all, granting unrestricted tool access. This is privilege escalation via configuration.

2. `RunConfig.safety_layer` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs:1952`:
   ```rust
   pub safety_layer: Option<SafetyLayer>,
   ```
   `RunConfig::Default` at line 2148-2196 sets `safety_layer: None` (line 2190). Tests and any struct literal using `..Default::default()` (e.g. event_loop.rs:23103-23105) therefore have zero safety enforcement. `from_roko_config` at line 2001 correctly populates the field, but this is not enforced at compile time.

3. Pre-dispatch safety gate at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs:10122-10123`:
   ```rust
   if let Some(ref safety) = ctx.config.safety_layer {
   ```
   When `safety_layer` is `None`, path escape detection, budget exhaustion, taint validation, and corrigibility enforcement are all silently skipped.

4. Post-dispatch safety gate at `event_loop.rs:3035`:
   ```rust
   if let Some(ref safety) = config.safety_layer {
   ```
   Same pattern. Secret leak detection and severity-based blocking are skipped.

5. Hermes provider adapter at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/hermes.rs:28-115`. `create_agent` constructs `HermesHttpAgent`, `HermesAcpAgent`, or `HermesOneShotAgent` with no `SafetyLayer`. None of these agent types carry a safety field. The adapter file imports nothing from `crate::safety`.

6. OpenClaw provider adapter at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openclaw.rs:28-90`. Same pattern: `OpenClawAcpAgent` and `OpenClawInferAgent` are constructed without any `SafetyLayer`. The file imports nothing from `crate::safety`.

7. `SafetyLayer::permissive()` at `safety/mod.rs:448-450` is correctly gated `#[cfg(test)]`. `AgentContract::permissive` at `crates/roko-agent/src/safety/contract.rs:165` is `pub` without any `#[cfg(test)]` gate — it is documented as being "for tests and adapter shims" but is not enforced by the compiler.

8. `AgentContract::restricted` at `contract.rs:181-200` is the correct deny-all fallback: `allowed_tools: Some(Vec::new())`, `NoNetworkAccess`, `MaxTokensPerTurn(4000)`, `MaxToolCallsPerTurn(10)`, `MaxConsecutiveFailures(3)`, `MaxCostPerTurn(0.50)`. The issue is in item 1 above, where this restricted fallback's deny-all is subsequently cleared.

## Implementation Plan

### Step 1: Fix `contract_for_role` to not clear deny-all on budget-only overrides

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs`

At lines 1282-1289, change the condition so it only clears `allowed_tools` when the role has an **explicit tools whitelist** (`self.role_tools.contains_key(role)`), not merely any override entry:

```rust
// Before (lines 1282-1289):
if contract
    .allowed_tools
    .as_ref()
    .is_some_and(|t| t.is_empty())
    && (self.role_tools.contains_key(role) || self.role_overrides.contains_key(role))
{
    contract.allowed_tools = None;
}

// After:
if contract
    .allowed_tools
    .as_ref()
    .is_some_and(|t| t.is_empty())
    && self.role_tools.contains_key(role)
{
    // Only defer to the TOML tools whitelist when an explicit tools list
    // was configured. A role with only budget/alias overrides keeps the
    // deny-all contract.
    contract.allowed_tools = None;
    tracing::info!(
        role,
        "contract: cleared deny-all allowlist; TOML tools whitelist is binding"
    );
}
```

Update the comment above the block (lines 1276-1281) to reflect the tightened semantics.

Add a regression test in the `#[cfg(test)]` module of `safety/mod.rs`:

```rust
#[test]
fn budget_only_override_keeps_deny_all_contract() {
    // A role with only a budget override (no tools key) must keep the deny-all
    // contract, not gain unrestricted tool access.
    let mut config = RokoConfig::default();
    config.agent.roles.insert(
        "custom-role".to_string(),
        RoleOverride {
            budget: Some(BudgetConfig { max_cost_usd_per_turn: Some(1.0), ..Default::default() }),
            tools: None,
            ..Default::default()
        },
    );
    let layer = SafetyLayer::from_config(&config);
    let contract = layer.contract_for_role("custom-role");
    assert!(
        contract.allowed_tools.as_ref().is_some_and(|t| t.is_empty()),
        "budget-only role must keep deny-all allowed_tools, got: {:?}",
        contract.allowed_tools
    );
}

#[test]
fn explicit_tools_override_defers_to_toml_whitelist() {
    let mut config = RokoConfig::default();
    config.agent.roles.insert(
        "tool-role".to_string(),
        RoleOverride {
            tools: Some(vec!["bash".to_string()]),
            ..Default::default()
        },
    );
    let layer = SafetyLayer::from_config(&config);
    let contract = layer.contract_for_role("tool-role");
    // None means the TOML whitelist is binding (not the contract's empty list).
    assert!(
        contract.allowed_tools.is_none(),
        "tools-configured role must defer to TOML whitelist, got: {:?}",
        contract.allowed_tools
    );
}
```

### Step 2: Make `RunConfig.safety_layer` non-optional

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs`

Change line 1952 from:
```rust
pub safety_layer: Option<SafetyLayer>,
```
To:
```rust
pub safety_layer: SafetyLayer,
```

Update `Default::default()` at line 2190 from:
```rust
safety_layer: None,
```
To:
```rust
safety_layer: SafetyLayer::with_defaults(),
```

Update `from_roko_config` at line 2069-2138. The existing logic:
```rust
let safety_layer = SafetyLayer::from_config(&roko_config);
// ...
safety_layer: Some(safety_layer),
```
becomes:
```rust
safety_layer: SafetyLayer::from_config(&roko_config),
```

Fix every compile error caused by the type change. The two `if let Some(ref safety) = ...` guards must be removed.

### Step 3: Remove the `if let Some` safety guards in event_loop.rs

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

At line 10123, change:
```rust
if let Some(ref safety) = ctx.config.safety_layer {
    let effective_safety = safety.clone().with_contract(effective_contract.clone());
    // ...
}
```
To:
```rust
let effective_safety = ctx.config.safety_layer.clone().with_contract(effective_contract.clone());
// ... (call unconditionally, remove the outer `if let`)
```

At line 3035, apply the same change:
```rust
// Before:
if let Some(ref safety) = config.safety_layer {
    // ...
}
// After: call safety methods unconditionally
let effective_safety = config.safety_layer.clone().with_contract(...);
```

Adjust indentation and any early-return logic that was inside the `if let` blocks.

### Step 4: Add `SafetyLayer` to Hermes and OpenClaw output scrubbing

Files:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/hermes.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openclaw.rs`

The agents returned by these adapters run external processes. Roko cannot control what tools the external process calls, but it can scrub secrets from the process's stdout/stderr before the output is passed to the caller.

The reference implementation is `ExecAgent` at `crates/roko-agent/src/exec.rs:46,56`, which carries a `safety: SafetyLayer` field and applies `safety.scrub_output(&content)` before returning results.

For each of `HermesHttpAgent`, `HermesAcpAgent`, `HermesOneShotAgent`, `OpenClawAcpAgent`, `OpenClawInferAgent`:

1. Add a `safety: SafetyLayer` field.
2. Add a builder method `with_safety(safety: SafetyLayer) -> Self`.
3. In the `Agent::run` implementation, after receiving the external process output, call `self.safety.scrub_output(&content)` and use the scrubbed result.

In the provider adapters (`hermes.rs`, `openclaw.rs`), pass `SafetyLayer::with_defaults()` when constructing each agent type. The `AgentOptions` passed to `create_agent` does not currently carry a `SafetyLayer`, so use the default hardened layer. If a task-specific contract is needed, that requires a larger `ProviderAdapter` trait change (explicitly out of scope here).

### Step 5: Gate `AgentContract::permissive` with `#[cfg(test)]`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contract.rs`

At line 165, `pub fn permissive` is public without a `#[cfg(test)]` gate. The existing doc comment says it is "retained for tests and adapter shims." Verify no production (non-test) code calls it:

```bash
grep -rn "AgentContract::permissive" crates/ --include='*.rs' | grep -v '#\[cfg(test\|fn permissive\|^.*//.*permissive'
```

If only test code calls it, add `#[cfg(test)]` to the function. If production code calls it, audit each call site and replace with `AgentContract::load_for_role` or `AgentContract::restricted`.

### Step 6: Add regression test for SafetyLayer non-optionality

File: `crates/roko-cli/src/runner/types.rs` or `crates/roko-cli/tests/`

```rust
#[test]
fn run_config_default_has_active_safety_layer() {
    let config = RunConfig::default();
    // SafetyLayer::with_defaults() is the fail-closed default.
    // We can't easily call pre_dispatch_check here, but we can verify
    // that the field is present and has restrictive defaults.
    let _safety = &config.safety_layer; // compile-time: field is non-optional
    // The default layer should have sandbox level None (not permissive).
    // Add stronger assertions if SafetyLayer exposes an introspection method.
}
```

## Acceptance Criteria

1. A role with only `[agent.roles.X.budget]` config (no `tools` key) receives a deny-all contract that blocks all tool calls; the `allowed_tools` field remains `Some(vec![])`.
2. A role with `[agent.roles.X.tools]` config correctly defers tool-access control to the TOML whitelist; the `allowed_tools` field is cleared to `None`.
3. `RunConfig.safety_layer` is type `SafetyLayer` (not `Option<SafetyLayer>`); the project compiles.
4. `RunConfig::default()` initializes `safety_layer` with `SafetyLayer::with_defaults()`.
5. Pre-dispatch and post-dispatch safety checks in `event_loop.rs` run unconditionally; neither is guarded by `if let Some(ref safety)`.
6. `HermesHttpAgent`, `HermesAcpAgent`, `HermesOneShotAgent`, `OpenClawAcpAgent`, and `OpenClawInferAgent` all apply `SafetyLayer::with_defaults()` output scrubbing before returning results.
7. `AgentContract::permissive` is either `#[cfg(test)]`-gated or has zero non-test callers.
8. `cargo test --workspace` passes with no regressions.
9. Regression tests from Steps 1 and 6 are present and pass.

## Verification Checklist

- [ ] Remove the `|| self.role_overrides.contains_key(role)` disjunction from `contract_for_role`
- [ ] Run `cargo test -p roko-agent` and confirm the two new tests pass
- [ ] Change `RunConfig.safety_layer` to non-optional; run `cargo build --workspace` to find all type errors
- [ ] Fix every type error (struct literals, `if let Some`, callers)
- [ ] Run `cargo test --workspace` to confirm no regressions
- [ ] Add `SafetyLayer::with_defaults()` output scrubbing to Hermes and OpenClaw agent types
- [ ] Search for `AgentContract::permissive` calls outside `#[cfg(test)]` and gate or remove them
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings`

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-agent/src/safety/mod.rs` | Tighten `contract_for_role` condition; add two regression tests |
| `crates/roko-cli/src/runner/types.rs` | Change `safety_layer: Option<SafetyLayer>` to `SafetyLayer`; update `Default` and `from_roko_config` |
| `crates/roko-cli/src/runner/event_loop.rs` | Remove `if let Some(ref safety)` guards at lines 10123 and 3035; call safety methods unconditionally |
| `crates/roko-agent/src/hermes/` | Add `safety: SafetyLayer` to agent structs; apply `scrub_output` |
| `crates/roko-agent/src/openclaw/` | Add `safety: SafetyLayer` to agent structs; apply `scrub_output` |
| `crates/roko-agent/src/provider/hermes.rs` | Pass `SafetyLayer::with_defaults()` when constructing agent types |
| `crates/roko-agent/src/provider/openclaw.rs` | Pass `SafetyLayer::with_defaults()` when constructing agent types |
| `crates/roko-agent/src/safety/contract.rs` | Add `#[cfg(test)]` to `AgentContract::permissive` if no production callers exist |
