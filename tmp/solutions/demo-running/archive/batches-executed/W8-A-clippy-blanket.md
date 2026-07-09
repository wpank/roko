# W8-A: Remove Blanket Clippy Suppression in main.rs

**Priority**: P2 — code health
**Effort**: 2-4 hours (lots of clippy fixes)
**Files to modify**: main.rs + files clippy flags
**Dependencies**: None

## Problem

`crates/roko-cli/src/main.rs` lines 10-20 suppress ALL clippy lints:
```rust
#![allow(clippy::too_many_lines)]
#![cfg_attr(
    clippy,
    allow(
        clippy::all,
        clippy::pedantic,
        clippy::nursery,
        clippy::restriction,
        missing_docs
    )
)]
```

This hides real issues. CI uses `-D warnings` so this blanket allow is the only thing preventing failures.

## Fix

### Step 1: Remove the blanket suppression

```rust
// KEEP (legitimate for a 3000+ line CLI):
#![allow(clippy::too_many_lines)]
#![allow(missing_docs)]  // CLI doesn't need full docs

// REMOVE the cfg_attr block entirely
```

### Step 2: Run clippy and fix issues

```bash
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | head -100
```

This will produce many warnings. Common categories:
- `clippy::needless_pass_by_value` → change to `&T`
- `clippy::redundant_closure` → replace with function reference
- `clippy::single_match` → use `if let`
- `clippy::unused_self` → make function associated
- `clippy::too_many_arguments` → add `#[allow]` on specific functions
- `clippy::module_name_repetitions` → rename or `#[allow]` per-item

### Step 3: Allow specific lints where needed

For legitimate cases (e.g., functions that genuinely need many arguments), add per-item allows:
```rust
#[allow(clippy::too_many_arguments)]
fn dispatch_agent_with(/* 8 params */) { ... }
```

### Strategy: Do NOT fix every lint

For a 3000+ line file, focus on:
1. **Fix** actual bugs caught by clippy
2. **Fix** easy mechanical transforms (redundant closures, single match, etc.)
3. **Allow** per-item for legitimate cases (too_many_arguments on complex functions)
4. **Keep** `#![allow(clippy::too_many_lines)]` at file level (legitimate)

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-A-clippy-blanket.md and implement all changes. Remove the cfg_attr(clippy, allow(...)) block in main.rs lines 10-20. Keep #![allow(clippy::too_many_lines)] and #![allow(missing_docs)]. Then this is the ONE batch that needs iterative clippy: run cargo clippy -p roko-cli --no-deps 2>&1 | head -200 and fix the warnings. Use per-item #[allow] for legitimate cases (too_many_arguments on complex functions). Do NOT run cargo test — tests are deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 7+8 batches together. Do not commit individually.

## Special Note

This is the ONE batch that requires iterative `cargo clippy` because removing the blanket suppression will expose many warnings that need targeted fixes. All other batches should NOT run cargo commands.

## Checklist

- [ ] Remove `#![cfg_attr(clippy, allow(...))]` block
- [ ] Keep `#![allow(clippy::too_many_lines)]` and `#![allow(missing_docs)]`
- [ ] Run clippy and fix/allow all warnings
- [ ] Actual bugs fixed, not just suppressed
- [ ] Per-item `#[allow]` for legitimate cases
- [ ] Pre-commit checks pass
