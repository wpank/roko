# W5-D: Refuse Silent Fallback to Cat Agent

**Priority**: P1 — prevents confusion
**Effort**: 15 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

Running `roko` without `roko.toml` or valid provider prints a warning then enters chat with a `cat` agent that just echoes input. User thinks roko is broken because it doesn't respond intelligently.

## Fix

Refuse to enter chat mode without a valid provider. Print an actionable error message.

### Find the fallback code

```bash
grep -rn 'cat\|fallback\|NeedsSetup\|no.*provider' crates/roko-cli/src/chat*.rs crates/roko-cli/src/main.rs | head -20
```

Find where the chat mode decides to use a fallback/cat agent when no provider is configured. This is likely in the chat entry point or in the auth detection → session creation path.

### Change

Where the code currently falls back to cat/echo:
```rust
// BEFORE:
AuthMethod::NeedsSetup => {
    eprintln!("warning: no provider configured, using cat agent");
    // ... starts cat agent
}

// AFTER:
AuthMethod::NeedsSetup => {
    eprintln!("error: no LLM provider configured.\n");
    eprintln!("To get started, either:");
    eprintln!("  1. Run `roko init` to create a workspace with default config");
    eprintln!("  2. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, or ZAI_API_KEY");
    eprintln!("  3. Edit roko.toml to configure a provider");
    eprintln!("\n  hint: run `roko doctor` to diagnose your setup");
    std::process::exit(1);
}
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W5-D-cat-fallback-refuse.md and implement all changes described in it. Find the cat/NeedsSetup fallback in chat entry code and replace with an actionable error message + exit. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 5 batches together. Do not commit individually.

## Checklist

- [x] Find where cat/echo fallback is triggered
- [x] Replace with actionable error message + exit
- [x] Message suggests: roko init, env vars, roko.toml, roko doctor
- [ ] Verify: no provider → error message (not broken chat)
- [ ] Pre-commit checks pass
