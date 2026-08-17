# Error Recovery Built But Not Wired

## Problem

`classify_agent_crash()` and `recovery_hint()` exist in the codebase but are never called
from the main execution paths. When agents crash, the user sees raw error text instead of
actionable recovery suggestions.

Additionally, `roko.toml` parse failures silently fall back to defaults instead of warning
the user, and gate-failure replan is not wired in runner v2.

## Root Cause

### A. `classify_agent_crash` + `recovery_hint` never called

**File:** `crates/roko-agent/src/error_recovery.rs`

Functions exist:
```rust
pub fn classify_agent_crash(error: &str) -> CrashClass { ... }
pub fn recovery_hint(class: &CrashClass) -> &str { ... }
```

These classify errors into categories (RateLimit, ContextOverflow, ToolFailed, AuthExpired,
ModelUnavailable, etc.) and provide recovery hints. But neither `do_cmd.rs`, `orchestrate.rs`,
nor `bridge_events.rs` call them. Errors propagate as raw `anyhow::Error` strings.

### B. Silent `roko.toml` fallback

**File:** `crates/roko-core/src/config/mod.rs`

```rust
pub fn load_config(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}
```

If `roko.toml` has a syntax error, the user gets silently downgraded to defaults. No warning,
no error message. The wrong model, wrong provider, wrong settings — and no indication why.

### C. Gate-failure replan not in runner v2

**File:** `crates/roko-cli/src/orchestrate.rs`

`build_gate_failure_plan_revision()` is called when a gate fails in the main orchestration
loop, generating a fix plan. But runner v2 (`PlanRunner` in `roko-orchestrator`) doesn't
call this — gate failures in runner v2 just mark the task as failed and move on.

## Fix

### Fix 1: Wire error recovery into do_cmd.rs (~15 min)

**File:** `crates/roko-cli/src/commands/do_cmd.rs`

After agent dispatch failure:
```rust
match agent_exec::run(&options).await {
    Ok(result) => { /* ... */ },
    Err(e) => {
        let class = classify_agent_crash(&e.to_string());
        let hint = recovery_hint(&class);
        eprintln!("Agent failed: {e}");
        eprintln!("Recovery: {hint}");
        // For RateLimit: auto-retry with backoff
        // For AuthExpired: prompt to refresh key
        // For ContextOverflow: suggest smaller context
    }
}
```

### Fix 2: Warn on config parse failure (~5 min)

**File:** `crates/roko-core/src/config/mod.rs`

```rust
pub fn load_config(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("WARNING: Failed to parse {}: {e}", path.display());
                eprintln!("Using default configuration. Run `roko config validate` to fix.");
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}
```

### Fix 3: Wire replan into runner v2 (~20 min)

**File:** `crates/roko-orchestrator/src/runner.rs`

After gate failure, call `build_gate_failure_plan_revision()` and append the fix task to the
current plan's remaining tasks.

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/do_cmd.rs` | Wire `classify_agent_crash` + `recovery_hint` |
| `crates/roko-acp/src/bridge_events.rs` | Wire error recovery for ACP dispatch |
| `crates/roko-core/src/config/mod.rs` | Warn on TOML parse failure |
| `crates/roko-orchestrator/src/runner.rs` | Wire gate-failure replan |

## Priority

**P1** — Error recovery is the difference between "it broke, try again" and "it broke because
X, here's how to fix it." The code exists, it just needs 15 minutes of wiring.
