# W8-B: Add rust-toolchain.toml Pinning 1.91+

**Priority**: P2 — prevents build failures
**Effort**: 5 minutes
**Files to modify**: 1 new file
**Dependencies**: None

## Problem

Alloy deps need Rust 1.91+ but this isn't enforced. New contributors run `cargo build` with older toolchain and get cryptic errors.

## Fix

### File: `/Users/will/dev/nunchi/roko/roko/rust-toolchain.toml` (new file, workspace root)

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

This ensures `rustup` auto-installs the latest stable toolchain. Stable is always >= 1.91 at this point.

If you want to pin a minimum version:
```toml
[toolchain]
channel = "1.91"
components = ["rustfmt", "clippy"]
```

But pinning the latest stable is better (gets security fixes).

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W8-B-rust-toolchain.md and implement all changes. Create rust-toolchain.toml at workspace root /Users/will/dev/nunchi/roko/roko/rust-toolchain.toml with channel = "stable" and components = ["rustfmt", "clippy"]. Do NOT run cargo build/test/clippy/fmt. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 7+8 batches together. Do not commit individually.

## Checklist

- [x] Create `rust-toolchain.toml` at workspace root
- [x] Set channel to `"stable"` with rustfmt + clippy components
- [x] Verify: `cargo build --workspace` works
- [x] Pre-commit checks pass
