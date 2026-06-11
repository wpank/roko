# 15: bare_mode Kills All Slash Commands

## Problem

When `bare_mode = true` in either `roko.toml` or `~/.roko/config.toml`, the ACP strips all slash commands except 8 basic ones. Typing `/prd-idea`, `/plan-generate`, `/build`, `/test`, etc. in Zed returns "not supported".

## Root Cause

`crates/roko-acp/src/session.rs:1359`:
```rust
const BARE_MODE_COMMANDS: &[&str] = &[
    "status", "doctor", "config", "help",
    "research", "search", "enhance-prd", "analyze",
];
```

`build_slash_commands(bare_mode: bool)` filters the 50+ registered commands down to just those 8 when `bare_mode = true`.

Both config files had `bare_mode = true`:
- `roko.toml` line 18: `bare_mode = true`
- `~/.roko/config.toml` line 5: `bare_mode = true`

The project config overrides the global config, so even fixing `~/.roko/config.toml` alone doesn't help.

## Fix Applied (2026-05-06)

Changed both files to `bare_mode = false`.

## What Should Have Prevented This

1. **Don't default to bare mode**: `bare_mode` should default to `false`. It was originally meant for non-roko workspaces where PRD/plan/knowledge commands don't apply, but it blocks too much.

2. **Show which mode is active**: When the ACP starts, log (and optionally tell the user):
   ```
   ACP mode: bare (8 commands) — set bare_mode = false for full 50+ commands
   ```

3. **Dynamic bare mode**: Instead of a config bool, detect bare mode automatically:
   - If `.roko/` exists → full mode
   - If no `.roko/` → bare mode (but still show build/test/clippy which work anywhere)

4. **The bare mode allowlist is too restrictive**: Even in bare mode, commands like `/build`, `/test`, `/clippy`, `/plan-generate` should work — they don't depend on roko workspace state. The allowlist should be:
   ```rust
   const BARE_MODE_COMMANDS: &[&str] = &[
       "status", "doctor", "config", "help",
       "research", "search", "enhance-prd", "analyze",
       "build", "test", "clippy", "fmt", "gate",  // verification always works
       "plan-generate", "run", "express",           // agent dispatch works anywhere
   ];
   ```

## Files

| File | Change |
|------|--------|
| `crates/roko-acp/src/session.rs:1359` | Expand `BARE_MODE_COMMANDS` or auto-detect bare mode |
| `roko.toml` | `bare_mode = false` |
| `~/.roko/config.toml` | `bare_mode = false` |
