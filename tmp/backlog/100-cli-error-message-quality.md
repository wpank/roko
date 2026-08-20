# 100 — CLI Error Message Quality: Recovery Hints, Exit Code Constants, and tracing Migration

**Priority**: P2 — UX; generic errors prevent users from self-diagnosing failures without reading source code
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli`
**Depends on**: None

---

## Background

The roko CLI has well-written errors in a few places — `auth_detect.rs:190-202` prints a three-option setup guide when no provider is configured, and `main.rs:3343-3348` prints a similar block for the unconfigured provider case — but these are exceptions. Most error paths tell the user what failed without suggesting what to do about it.

There are also two related code hygiene gaps. First, several files call `std::process::exit(1)` directly rather than the named exit code constants defined at `main.rs:79-85` (`EXIT_SUCCESS = 0`, `EXIT_FAILURE = 1`, `EXIT_AGENT_FAILURE = 1`, `EXIT_SYSTEM_ERROR = 2`). This makes it hard to audit exit behavior or change the codes later. Second, some warning and error conditions that should appear in structured logs use `eprintln!` instead of `tracing::warn!` or `tracing::error!`, making them invisible to log files and `ROKO_LOG_FORMAT=json` consumers.

The goal of this item is not to fix every error message in the codebase (there are hundreds). It is to fix the 15-20 highest-impact user-facing paths — the ones a first-time user is most likely to hit — and establish a pattern that future work can follow.

## Current State

### Issue 1: Workflow halt/cancel/fail messages give no recovery hint

**File:** `crates/roko-cli/src/commands/util.rs`, lines 357-368

```rust
if !report.success {
    match roko_cli::run::workflow_report_outcome(&report) {
        Some(roko_core::WorkflowOutcome::Halted { reason }) => {
            eprintln!("error: workflow halted: {reason}");
        }
        Some(roko_core::WorkflowOutcome::Cancelled) => {
            eprintln!("error: workflow cancelled");
        }
        Some(roko_core::WorkflowOutcome::Success { .. }) | None => {
            eprintln!("error: workflow failed");
        }
    }
}
```

None of these messages suggest `--resume-plan`, checking `.roko/roko.log`, or running `roko doctor`.

### Issue 2: `custody.rs` uses `std::process::exit(1)` instead of named constants

**File:** `crates/roko-cli/src/custody.rs`, lines 155-161 and 253-264

```rust
// Line 155-160 (cmd_custody_show):
if records.is_empty() {
    eprintln!("No custody records found at {}", layout.custody_log().display());
    std::process::exit(1);   // ← should be EXIT_FAILURE
}

// Line 253-264 (cmd_custody_verify):
if !log_path.exists() {
    eprintln!("No custody log found at {}", log_path.display());
    std::process::exit(1);   // ← should be EXIT_FAILURE
}
// ...
if lines.is_empty() {
    eprintln!("Custody log is empty.");
    std::process::exit(1);   // ← should be EXIT_FAILURE
}
```

The exit code constants are defined in `main.rs` (lines 79-85) but are not visible to `custody.rs` because they are local to `main.rs`. `custody.rs` must either be changed to use `return Err(...)` / return a non-zero `i32`, or the constants must be moved to a shared location.

**File:** `crates/roko-cli/src/main.rs`, line 3349

```rust
std::process::exit(1);   // ← in the unconfigured-provider detection block
```

### Issue 3: `unified.rs` uses bare `eprintln!("error: {e:#}")` with no context

**File:** `crates/roko-cli/src/unified.rs`, lines 154 and 163

```rust
// Line 154:
eprintln!("error: {e:#}");
// Line 163:
eprintln!("error: {e:#}");
```

Both are user-facing errors (ChatAgentSession init failure and single-turn dispatch failure) with no hint about what to check. The init failure at line 154 is accompanied by `tracing::warn!` but the message is still a generic `error:` with no suggestion.

### Issue 4: `prd.rs` uses `eprintln!` for structured log events

**File:** `crates/roko-cli/src/prd.rs`, lines 963 and 1582

```rust
// Line 963: audit event append failure
eprintln!("warning: failed to append PRD publish audit event: {err:#}");

// Line 1582: PRD frontmatter update failure (already has a tracing::warn! at line 1583)
eprintln!("warning: failed to update PRD plans_generated: {err}");
tracing::warn!(slug = %slug, error = %err, "failed to update PRD plans_generated field");
```

Line 963 has no corresponding `tracing::warn!` call, so the failure is invisible to structured logs. Line 1582 is duplicated (both `eprintln!` and `tracing::warn!`) — the `eprintln!` should be removed and the `tracing::warn!` kept.

Note: `prd.rs:1038-1041` is already correct — it prints an actionable error with a recovery hint (`"Run 'roko prd plan <slug>' manually"`). That pattern is the target for the other sites.

### Issue 5: Trigger "not found" errors don't suggest the list command

**File:** `crates/roko-cli/src/commands/trigger.rs`, lines 171, 288, 364

```rust
eprintln!("trigger '{name}' not found at {}", path.display());
```

This message appears three times (in `cmd_show`, `cmd_fire`, and `cmd_delete`). None suggest `roko trigger list` or `roko config events` to see available triggers.

Also at line 301:
```rust
eprintln!("trigger '{name}' is disabled; enable it first");
```
Does not suggest the `roko trigger enable <name>` command.

### Issue 6: `unified.rs` session failure has no actionable hint

**File:** `crates/roko-cli/src/unified.rs`, lines 152-156

The error on session init failure should suggest `roko doctor` for diagnosis, since the most common cause is a misconfigured provider.

## Implementation Plan

### B1. Add recovery hints to workflow outcome messages

In `crates/roko-cli/src/commands/util.rs`, lines 357-368, add hint lines after each error:

```rust
Some(roko_core::WorkflowOutcome::Halted { reason }) => {
    eprintln!("error: workflow halted: {reason}");
    eprintln!("  → Check logs: .roko/roko.log");
    eprintln!("  → Resume:     roko plan run <dir> --engine runner-v2 --resume-plan");
    eprintln!("  → Diagnose:   roko doctor");
}
Some(roko_core::WorkflowOutcome::Cancelled) => {
    eprintln!("error: workflow cancelled");
    eprintln!("  → Resume:     roko plan run <dir> --engine runner-v2 --resume-plan");
}
Some(roko_core::WorkflowOutcome::Success { .. }) | None => {
    eprintln!("error: workflow failed");
    eprintln!("  → Check logs: .roko/roko.log");
    eprintln!("  → Diagnose:   roko doctor");
}
```

### B2. Move exit code constants to a shared location

Create `crates/roko-cli/src/exit_codes.rs` with:

```rust
//! Named CLI exit codes.

/// Successful execution.
pub const EXIT_SUCCESS: i32 = 0;
/// Agent or gate failure (logical error in the build).
pub const EXIT_FAILURE: i32 = 1;
/// Agent or gate failure (alias for EXIT_FAILURE, kept for backward compat).
pub const EXIT_AGENT_FAILURE: i32 = 1;
/// System error (I/O, config, infrastructure).
pub const EXIT_SYSTEM_ERROR: i32 = 2;
```

Add `pub mod exit_codes;` to `crates/roko-cli/src/lib.rs`.

In `main.rs`, replace the local constant definitions with `use roko_cli::exit_codes::*;`.

In `custody.rs`, replace `std::process::exit(1)` with `return Ok(EXIT_FAILURE)` after changing the function signatures to return `Result<i32>` (or `anyhow::bail!` if the function returns `Result<()>` — check the actual return types).

In `main.rs:3349`, replace `std::process::exit(1)` with `std::process::exit(EXIT_FAILURE)`.

### B3. Add actionable hint in `unified.rs` session init failure

In `crates/roko-cli/src/unified.rs`, lines 152-156:

```rust
Err(e) => {
    tracing::warn!("ChatAgentSession init failed: {e:#}");
    eprintln!("error: failed to initialize agent session: {e:#}");
    eprintln!("  → Run `roko doctor` to diagnose provider configuration.");
    return Ok(1);
}
```

For the dispatch failure at line 160-164:

```rust
Err(e) => {
    eprintln!("error: agent dispatch failed: {e:#}");
    eprintln!("  → Check .roko/roko.log for details.");
    return Ok(1);
}
```

### B4. Fix `prd.rs` eprintln! / tracing duplication

In `crates/roko-cli/src/prd.rs`, line 963:
Replace `eprintln!("warning: failed to append PRD publish audit event: {err:#}")` with:
```rust
tracing::warn!(error = %err, "failed to append PRD publish audit event");
```

In `crates/roko-cli/src/prd.rs`, line 1582:
Remove the `eprintln!` line and keep only the `tracing::warn!` at line 1583.

### B5. Add "try listing" hints to trigger "not found" messages

In `crates/roko-cli/src/commands/trigger.rs`, all three "not found" sites (lines 171, 288, 364):

```rust
eprintln!("trigger '{name}' not found");
eprintln!("  → Run `roko trigger list` to see available triggers.");
```

Remove the path from the error message (it's noise for the user; they didn't choose the path directly).

At line 301 ("disabled"):

```rust
eprintln!("trigger '{name}' is disabled");
eprintln!("  → Run `roko trigger enable {name}` to enable it first.");
```

## Acceptance Criteria

1. `roko run --help` still works (no broken imports from exit code refactor).
2. `cargo test -p roko-cli` passes all existing tests.
3. Workflow halt/cancel/fail messages each include at least one `→` recovery hint.
4. `std::process::exit(1)` no longer appears in `custody.rs` or at `main.rs:3349`.
5. `eprintln!` in `prd.rs:963` is replaced by `tracing::warn!`; the duplicate `eprintln!` at `prd.rs:1582` is removed.
6. Trigger "not found" messages include a `→ Run 'roko trigger list'` hint.
7. `cargo clippy --workspace --no-deps -- -D warnings` passes clean.
8. New test: when a workflow reports `Cancelled`, `eprintln` output contains "Resume:" and "resume-plan".

### Not in Scope

- Localization or i18n of error messages
- Colorized error output
- Machine-parseable error codes
- Rewriting all 200+ error sites (focus on the ~15 listed above)
- Changing `eprintln!` calls that are intentionally user-facing status updates (not errors)

## Verification Checklist

- [ ] `grep -n 'std::process::exit(1)' crates/roko-cli/src/custody.rs` returns no results
- [ ] `grep -n 'std::process::exit(1)' crates/roko-cli/src/main.rs | grep -v EXIT_FAILURE` returns no results
- [ ] `grep -n 'eprintln.*warning.*PRD publish audit' crates/roko-cli/src/prd.rs` returns no results
- [ ] `cargo test -p roko-cli 2>&1 | tail -5` shows all tests passing
- [ ] `cargo clippy --workspace --no-deps -- -D warnings 2>&1 | grep 'error\[' | wc -l` equals 0

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/exit_codes.rs` | Create: shared exit code constants |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` | Add `pub mod exit_codes;` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` | Use `exit_codes::*`; replace hardcoded `exit(1)` at line 3349 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/custody.rs` | Replace `std::process::exit(1)` with `return Ok(EXIT_FAILURE)` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs` | Add recovery hints to workflow halt/cancel/fail messages (lines 357-368) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/unified.rs` | Add `roko doctor` hint on session init failure (lines 152-156); add log hint on dispatch failure (lines 160-164) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` | Replace `eprintln!` at line 963 with `tracing::warn!`; remove duplicate `eprintln!` at line 1582 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/trigger.rs` | Add `→ Run 'roko trigger list'` hints at lines 171, 288, 364; add enable hint at line 301 |
