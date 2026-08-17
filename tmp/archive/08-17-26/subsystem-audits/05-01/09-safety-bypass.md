# 09 — Safety Layer: Permissive Defaults & Permission Bypass

## CRITICAL: 8 hardcoded `dangerously_skip_permissions = true` sites

**File:** `crates/roko-agent/src/provider/mod.rs:539-573`

The codebase has an explicit audit inventory documenting this:

```
Security audit inventory (PE_01):
- NEEDS FIX (hardcoded `true`):
  - crates/roko-agent/src/claude_cli_agent.rs:128
  - crates/roko-cli/src/runner/types.rs:1337
  - crates/roko-cli/src/runner/types.rs:1382
  - crates/roko-cli/src/agent_exec.rs:145
  - crates/roko-cli/src/serve_runtime.rs:547
  - crates/roko-cli/src/commands/plan.rs:394
  - crates/roko-serve/src/dispatch.rs:1824
  - crates/roko-acp/src/runner.rs:1763
```

Comment says: **"Default MUST be `false`. PE_02 will flip all NEEDS FIX sites."**

PE_02 never shipped. Every agent dispatched by roko runs with permissions bypassed.

---

## CRITICAL: Permissive safety fallback

**File:** `crates/roko-agent/src/safety/mod.rs:866-876`

```rust
fn contract_for_role(&self, role: &str) -> AgentContract {
    if self.role_overrides.contains_key(role) {
        return match AgentContract::load_for_role(role) {
            Ok(contract) => contract,
            Err(ContractLoadError::MissingAsset { .. }) => {
                tracing::debug!(
                    %role,
                    "no bundled contract for configured role; using permissive fallback"
                );
                AgentContract::permissive(role)  // allows everything
            }
```

When a role config exists but no YAML contract file is bundled, the system falls back to `permissive()` instead of `restricted()`. This means:
- All governance rules bypassed
- All tool calls allowed
- No invariant enforcement
- No cost limits
- Only a `debug!` log (not `warn!` or `error!`)

**CLAUDE.md says:** "Safety contracts enforcement: Partial — AgentContract wired but falls back to permissive default when YAML missing"

This is accurate documentation but the wrong default. It should fail closed.

---

## HIGH: Hallucination detector uses permissive default

**File:** `crates/roko-agent/src/safety/hallucination.rs:34-37`

```rust
pub fn permissive() -> Self {
    Self {
        known_tools: Vec::new(),  // Empty list = accepts ANY tool name
    }
}
```

All tests use `HallucinationDetector::permissive()`. The detector only validates parameter shapes, not whether tools exist. A hallucinating agent that invokes `rm_rf_everything` passes the check because the tool name list is empty.

---

## HIGH: Supervision default allows zero restarts

**File:** `crates/roko-runtime/src/process.rs:150-162`

```rust
impl Default for SupervisionStrategy {
    fn default() -> Self {
        Self::OneForOne {
            max_restarts: 0,      // No restarts allowed
            within_ms: 0,
            fallback_tier: "standard".into(),
        }
    }
}
```

If an agent crashes, it stays dead. No retry, no escalation. Combined with the permissive safety defaults, agents can fail in ways that are both dangerous and unrecoverable.

---

## MEDIUM: Tool cost tracking incomplete

**File:** `crates/roko-agent/src/safety/contract.rs:481-483`

```rust
// TODO(UX26): enforce cumulative per-turn spend once tool-cost
// accounting is threaded into ToolContext.
```

`MaxCostPerTurn` governance only checks estimated cost on individual calls, not cumulative. Agents can exceed budget by issuing many small tool calls.

---

## ROOT CAUSE

The safety system was designed with the right primitives (contracts, roles, hallucination detection, spend limits) but the **defaults are all permissive**. This pattern emerged because:

1. The runners needed agents to actually execute code during development
2. Restrictive defaults would break every test and dispatch
3. Nobody went back to flip the defaults after the wiring was done
4. PE_02 (the "flip to secure" task) was planned but never executed

**The safety layer is correctly wired but effectively disabled.**
