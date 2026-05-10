# ACP Slash Command Bugs: Wrong Flags, Missing Commands, Broken Dispatch

## Problem

Of ~39 ACP slash commands, 3 are broken with wrong CLI flags, 1 important command is missing,
and 6 are partially functional due to the tool alias bug (#15).

## Inventory

### Broken (3)

1. **`/plan-resume`** — sends `--resume` but CLI flag is `--resume-plan`
   - **File:** `crates/roko-acp/src/bridge_events.rs:3573`
   - Command built: `roko plan run <dir> --resume <snapshot>`
   - CLI expects: `roko plan run <dir> --resume-plan <snapshot>`
   - Result: flag ignored, plan restarts from scratch

2. **`/plan-run`** — doesn't pass `--model` from session
   - **File:** `crates/roko-acp/src/bridge_events.rs:3560`
   - The session has a `model` field but `/plan-run` doesn't include it
   - Result: plan runs with default model, ignoring user's model selection

3. **`/search`** — wrong API format (see doc 13)
   - Sends batch format, Perplexity expects flat query

### Missing (1)

4. **`/develop`** — not wired as ACP slash command
   - `crates/roko-cli/src/commands/develop.rs` exists and works from CLI
   - Should be: `roko develop "<prompt>" --yes` (auto-approve for ACP)
   - Currently users must use `/do` for the full pipeline
   - **File to add:** `crates/roko-acp/src/session.rs:1542-1600` (slash command definitions)
   - **File to add:** `crates/roko-acp/src/bridge_events.rs` (dispatch handler)

### Partially functional (6)

All affected by tool alias bug (#15) — zero tools on non-Claude models:

5. `/analyze` → text-only output, no files created
6. `/research` → text-only output, no research artifacts
7. `/enhance-prd` → text-only, no PRD modifications
8. `/enhance-plan` → text-only, no plan modifications
9. `/enhance-tasks` → text-only, no task modifications
10. `/prd-draft` → zero tools (separate bug: `allowed_tools: "none"`)

### Working (30)

`/do`, `/plan-generate`, `/plan-run`, `/plan-list`, `/plan-show`, `/plan-validate`,
`/prd-idea`, `/prd-list`, `/prd-status`, `/prd-plan`, `/prd-consolidate`,
`/status`, `/doctor`, `/init`, `/chat`, `/agent-list`, `/agent-start`, `/agent-stop`,
`/explain`, `/run`, `/knowledge-query`, `/knowledge-stats`, `/learn-all`,
`/learn-router`, `/learn-experiments`, `/learn-efficiency`, `/config-show`,
`/config-validate`, `/config-providers`, `/config-models`

## Fix

### Fix 1: `/plan-resume` flag name (~2 min)

**File:** `crates/roko-acp/src/bridge_events.rs:3573`

```rust
// Before:
args.push("--resume".to_string());
// After:
args.push("--resume-plan".to_string());
```

### Fix 2: `/plan-run` model passthrough (~5 min)

**File:** `crates/roko-acp/src/bridge_events.rs:3560`

```rust
if let Some(model) = &session.model {
    args.push("--model".to_string());
    args.push(model.clone());
}
```

### Fix 3: Wire `/develop` (~15 min)

**File:** `crates/roko-acp/src/session.rs`

Add to slash command definitions:
```rust
SlashCommand {
    name: "develop".to_string(),
    description: "Full pipeline: scope → plan → execute".to_string(),
    params: vec![SlashParam::required("prompt", "What to develop")],
}
```

**File:** `crates/roko-acp/src/bridge_events.rs`

Add dispatch handler:
```rust
"develop" => {
    let prompt = params.get("prompt").unwrap();
    let args = vec!["develop", prompt, "--yes"];
    run_cli_subprocess(&args, &session).await
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-acp/src/bridge_events.rs:3573` | Fix `--resume` → `--resume-plan` |
| `crates/roko-acp/src/bridge_events.rs:3560` | Add `--model` passthrough |
| `crates/roko-acp/src/session.rs` | Add `/develop` slash command definition |
| `crates/roko-acp/src/bridge_events.rs` | Add `/develop` dispatch handler |

## Priority

**P0** — `/plan-resume` being broken means resuming failed plans from ACP restarts them
from scratch, losing all progress. A 2-minute flag name fix.
