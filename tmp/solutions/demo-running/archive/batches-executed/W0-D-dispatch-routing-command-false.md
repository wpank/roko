# W0-D: Fix `dispatch_agent()` Routing When `command != "claude"`

**Priority**: P0 — `roko run` on Railway is completely broken for all models
**Effort**: 1 hour
**Files to modify**: 1-2 files
**Dependencies**: None

## Problem

On Railway, `roko run "Build a CLI calculator in Rust" --model glm51` fails because:

1. `config.agent.command = "false"` (Railway config disables CLI subprocess — there IS no Claude binary)
2. `dispatch_agent()` at `run.rs:1829` checks: `use_provider_routing = has_routing && config.agent.command == "claude"`
3. Since command is `"false"`, `use_provider_routing` is **false** even though providers are fully configured
4. All subsequent checks also fail: `"false" != "claude"`, `is_known_protocol_command("false")` is false
5. Falls to generic subprocess path (line 2043) which tries to execute `false` as a subprocess
6. The Unix `false` command immediately exits with code 1 — agent dispatch fails

This means **every `roko run` on Railway fails** regardless of which model is selected.

## Root Cause

The routing decision at `run.rs:1829` is fundamentally wrong. It gates provider routing on `command == "claude"`, but Railway deployments use `command = "false"` precisely because there IS no local Claude CLI — they rely entirely on API providers. The check should be about whether providers are configured, not about the command string.

## Exact Code to Change

### Fix 1: Rewrite the routing decision

**File**: `crates/roko-cli/src/run.rs` — line 1828-1829

**Current:**
```rust
    let has_routing = !routing_config.providers.is_empty() || !routing_config.models.is_empty();
    let use_provider_routing = has_routing && config.agent.command == "claude";
```

**New:**
```rust
    let has_routing = !routing_config.providers.is_empty() || !routing_config.models.is_empty();
    // Use provider routing when providers/models are configured.
    // Previously gated on `command == "claude"`, which broke Railway deployments
    // where command="false" (no local CLI binary — API-only).
    let use_provider_routing = has_routing;
```

### Fix 2: Guard the Claude-specific paths

The subsequent `else if` branches check for `config.agent.command == "claude"` and `is_known_protocol_command()`. With the routing decision fixed, these branches only trigger when there are NO configured providers, which is fine — they handle the legacy local-binary case.

No change needed to lines 1909-2042 — they're only reached when `use_provider_routing` is false (no providers configured), in which case falling back to CLI-based dispatch is correct.

### Fix 3: Handle `command = "false"` gracefully in fallback path

If somehow no providers are configured AND command is "false", the generic subprocess path should fail with a helpful message instead of silently executing `/usr/bin/false`.

**File**: `crates/roko-cli/src/run.rs` — before line 2043 (the final `else` branch)

**Add a check:**
```rust
    } else if config.agent.command == "false" || config.agent.command.is_empty() {
        anyhow::bail!(
            "No LLM providers configured and agent.command is '{}'. \
             Configure [providers] and [models] in roko.toml, or set agent.command \
             to a valid CLI binary (e.g., 'claude').",
            config.agent.command
        );
    } else {
        // ... existing generic subprocess path
```

## Verification

After this fix:
- `roko run "..." --model glm51` on Railway should route through provider routing (Path A)
- `roko run "..." --model gpt54-mini` on Railway should route through provider routing (Path A)
- Local dev with `command = "claude"` and no providers should still use Claude CLI path
- `command = "false"` with no providers should give a clear error message

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-D-dispatch-routing-command-false.md and implement all changes. The key fix is: change `use_provider_routing` at run.rs:1829 to not gate on `command == "claude"` — gate only on `has_routing`. Also add a graceful error when command is "false" and no providers are configured. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical pipeline fixes). Do not commit individually.

## Checklist

- [x] Change `use_provider_routing` to `has_routing` (remove `&& config.agent.command == "claude"`)
- [x] Add error for `command = "false"` with no providers configured
- [ ] Verify: `roko run` on Railway routes through provider routing (Path A)
- [ ] Verify: local `command = "claude"` still works when no providers configured
