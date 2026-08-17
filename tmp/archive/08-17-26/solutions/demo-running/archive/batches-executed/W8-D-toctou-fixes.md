# W8-D: Fix TOCTOU Race Conditions

**Priority**: P2 — correctness
**Effort**: 1-2 hours
**Files to modify**: Multiple (pattern fix)
**Dependencies**: None

## Problem

Pattern across codebase: `if path.exists() { fs::read(&path) }` — file may be deleted between check and read. This is a TOCTOU (Time-of-Check-Time-of-Use) race condition.

## Fix Pattern

Replace check-then-use with try-then-handle:

```rust
// BEFORE (TOCTOU):
if path.exists() {
    let content = fs::read_to_string(&path)?;
    // use content
}

// AFTER (atomic):
match fs::read_to_string(&path) {
    Ok(content) => {
        // use content
    }
    Err(e) if e.kind() == io::ErrorKind::NotFound => {
        // handle missing file
    }
    Err(e) => return Err(e.into()),
}
```

## How to Find All Instances

```bash
grep -rn '\.exists()' crates/ --include='*.rs' | grep -v target/ | grep -v test | head -50
grep -rn 'if.*\.is_file()' crates/ --include='*.rs' | grep -v target/ | grep -v test | head -50
```

Focus on cases where `.exists()` or `.is_file()` is followed by a read/open of the same path. NOT all `.exists()` calls are TOCTOU — some just check for the presence of a directory structure.

## Priority Instances

Focus on these high-risk patterns:
1. Config file loading: check exists then read
2. State file loading: check exists then read
3. Lock file operations: check exists then create
4. Plan discovery: check exists then parse

Lower priority (unlikely race):
- Directory creation: `create_dir_all` is already idempotent
- Template loading: files don't change at runtime

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-D-toctou-fixes.md and implement all changes. Search for .exists() + read patterns across crates/ (grep -rn '\.exists()' crates/ --include='*.rs' | grep -v target/ | grep -v test). Focus on high-risk cases: config file loading, state file loading, lock file operations, plan discovery. Convert to try-then-handle with NotFound error matching. Keep .exists() for directory checks. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 7+8 batches together. Do not commit individually.

## Checklist

- [x] Search for `.exists()` + read patterns
- [x] Convert high-risk instances to try-then-handle
- [x] Handle `NotFound` error specifically (not generic error)
- [x] Keep `.exists()` for directory checks (idempotent, not TOCTOU)
- [ ] Pre-commit checks pass
