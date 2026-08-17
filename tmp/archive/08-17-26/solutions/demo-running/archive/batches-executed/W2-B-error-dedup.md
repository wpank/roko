# W2-B: Fix Double-Printed Errors

**Priority**: P1 — makes demo presentable
**Effort**: 1-2 hours
**Files to modify**: 8 files
**Dependencies**: None

## Problem

Every error prints twice:
```
Error: missing field `role`
error: missing field `role`
```

## Root Cause

Command handlers use `eprintln!("error: ...")` AND return `Err(...)` which gets printed by the top-level error handler in `main()`. There are **80 eprintln! calls** across 8 command files.

## Audit of eprintln! Calls by File

| File | Count | Notes |
|------|-------|-------|
| `crates/roko-cli/src/commands/prd.rs` | 19 | Many are warnings (ok to keep), some are errors (remove) |
| `crates/roko-cli/src/commands/plan.rs` | 16 | Mix of errors and warnings |
| `crates/roko-cli/src/commands/util.rs` | 18 | Status display (ok) + errors (remove) |
| `crates/roko-cli/src/commands/auth.rs` | 10 | Auth error messages |
| `crates/roko-cli/src/commands/server.rs` | 6 | Server startup messages |
| `crates/roko-cli/src/commands/config_cmd.rs` | 5 | Config error messages |
| `crates/roko-cli/src/commands/learn.rs` | 3 | Learning display |
| `crates/roko-cli/src/commands/job.rs` | 3 | Job error messages |

## Strategy

**Keep**: `eprintln!` calls that are **warnings** (e.g., `"warning: ..."`) or **informational** (e.g., progress messages). These don't correspond to a returned `Err(...)`.

**Remove**: `eprintln!` calls that print **the same message** as a subsequently returned `Err(...)`. The pattern is:
```rust
// BEFORE (double-prints):
eprintln!("error: {msg}");
return Err(anyhow!("{msg}"));

// AFTER (single print):
return Err(anyhow!("{msg}"));
```

Also convert bare `eprintln!("error: ...")` followed by `return Ok(1)` to use `Err`:
```rust
// BEFORE (inconsistent):
eprintln!("error: {msg}");
return Ok(1);

// AFTER (consistent):
anyhow::bail!("{msg}");
```

## How to Implement

### Step 1: Search for the pattern

```bash
# Find all eprintln! calls that precede an Err return or Ok(1) return
grep -n 'eprintln!' crates/roko-cli/src/commands/*.rs
```

### Step 2: For each file, apply this mechanical transformation

For each `eprintln!("error: ...")` or `eprintln!("Error: ...")`:
1. Check if the next statement returns `Err(...)` with the same message → **remove the eprintln!**
2. Check if the next statement returns `Ok(1)` → **replace both with `anyhow::bail!(...)`**
3. If it's a `eprintln!("warning: ...")` → **keep it** (warnings are separate from error returns)

### Step 3: Ensure top-level error handler formats nicely

In `main.rs`, the top-level error handler should print errors with consistent formatting. Find the `main()` function's error handling (around line 1736):

```rust
// Make sure this prints nicely
Err(e) => {
    eprintln!("error: {e:#}");
    std::process::exit(EXIT_SYSTEM_ERROR);
}
```

The `#` alternate format includes the anyhow chain. This is the ONE place errors should be printed.

### Step 4: Special case — validate_before_run

In `plan.rs`, `validate_before_run()` uses `eprintln!` + `return Some(1)`. This function returns `Option<i32>`, not `Result`. Here, convert to return `Some(1)` only, and let the caller print the error:

```rust
// OR change validate_before_run to return Result and remove all its eprintln! calls
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-B-error-dedup.md and implement all changes described in it. Audit each eprintln! call in the 8 command files listed. Remove ones that duplicate a returned Err. Convert eprintln+Ok(1) to anyhow::bail!. Keep warning eprintln! calls (they're informational). Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Just make the code changes and mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 2 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation: errors appear exactly once; warnings still appear.

## Checklist

- [x] Audit all 80 eprintln! calls across 8 command files
- [x] Remove eprintln! calls that duplicate a returned Err
- [x] Convert eprintln! + Ok(1) patterns to anyhow::bail!
- [x] Keep warning eprintln! calls (they're informational, not errors)
- [x] Verify top-level error handler prints once with good formatting
- [ ] Test: errors appear exactly once
- [ ] Test: warnings still appear
- [ ] Pre-commit checks pass
