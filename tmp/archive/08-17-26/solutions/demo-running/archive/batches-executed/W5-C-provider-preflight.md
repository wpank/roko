# W5-C: Provider Binary + API Key Pre-Flight at Boot

**Priority**: P1 — prevents wasted time
**Effort**: 1 hour
**Files to modify**: 2 files
**Dependencies**: None

## Problem

Using `claude_cli` when `claude` is not installed leads to "spawn failed: No such file or directory" AFTER task context has been built (minutes wasted). API key issues only surface at dispatch time.

## Fix

Check provider binary and API key availability at boot, not at dispatch time.

### File: `crates/roko-cli/src/commands/plan.rs` (or wherever plan run starts)

Before entering the plan execution loop, validate providers:

```rust
fn preflight_providers(config: &RokoConfig) -> Result<()> {
    for (name, provider) in &config.providers {
        // Check API key
        if let Some(ref env_var) = provider.api_key_env {
            match std::env::var(env_var) {
                Ok(val) if val.is_empty() => {
                    anyhow::bail!(
                        "Provider '{}' requires {} but it is empty.\n  hint: export {}=<your-key>",
                        name, env_var, env_var
                    );
                }
                Err(_) => {
                    anyhow::bail!(
                        "Provider '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                        name, env_var, env_var
                    );
                }
                Ok(_) => {} // valid
            }
        }

        // Check binary for CLI-based providers
        if let Some(ref binary) = provider.command {
            if which::which(binary).is_err() {
                anyhow::bail!(
                    "Provider '{}' requires '{}' on PATH but it was not found.\n  hint: install {} or change provider in roko.toml",
                    name, binary, binary
                );
            }
        }
    }
    Ok(())
}
```

### Where to call it

1. **Before `plan run`**: In the plan run handler, after loading config but before dispatching any agents
2. **Before `prd plan` / `prd draft new`**: Before the LLM dispatch in PRD commands
3. **Before `roko chat`**: In the chat entry point

### Dependency: `which` crate

Add to `crates/roko-cli/Cargo.toml`:
```toml
which = "7"
```

Or use a simpler check without the dependency:
```rust
fn binary_on_path(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

## Also: Gate Dependencies Pre-Flight

Gates assume `cargo`, `git`, `clippy` are available. Add checks:

```rust
fn preflight_gate_deps() -> Vec<String> {
    let mut missing = Vec::new();
    for tool in &["cargo", "git"] {
        if which::which(tool).is_err() {
            missing.push(tool.to_string());
        }
    }
    // clippy is a cargo component, not a standalone binary
    if std::process::Command::new("cargo").args(["clippy", "--version"]).output()
        .map(|o| !o.status.success()).unwrap_or(true)
    {
        missing.push("clippy".to_string());
    }
    missing
}
```

Call before `plan run` and warn (don't fail — some gates may not need all tools):
```rust
let missing = preflight_gate_deps();
if !missing.is_empty() {
    eprintln!("warning: missing gate tools: {}. Some gates may fail.", missing.join(", "));
}
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-C-provider-preflight.md and implement all changes described in it. Create preflight_providers() and preflight_gate_deps() functions. Wire into plan run, prd plan, prd draft new, and roko chat entry points. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 5 batches together. Do not commit individually.

## Checklist

- [x] Create `preflight_providers()` function
- [x] Check API key env var is set and non-empty
- [x] Check provider binary is on PATH
- [x] Call before `plan run`, `prd plan`, `prd draft new`, `roko chat`
- [x] Create `preflight_gate_deps()` for cargo/git/clippy
- [x] Call before `plan run` (warn, don't fail)
- [x] Verify: missing binary fails fast with actionable message
- [x] Verify: missing API key fails fast with actionable message
- [ ] Pre-commit checks pass
