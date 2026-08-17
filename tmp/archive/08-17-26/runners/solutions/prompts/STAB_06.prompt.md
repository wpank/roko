# STAB_06: Remove `unsafe set_var` for --provider

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-06`](../ISSUE-TRACKER.md#stab-06)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.06
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Line 236 of `util.rs`:
```rust
unsafe { std::env::set_var("ROKO_PROVIDER", p) };
```
This is undefined behavior in multi-threaded Rust programs since 1.66. The tokio runtime
is already spawned at this point, making this unsafe.

Two additional `unsafe set_var` calls exist in `main.rs`:
- Line 2225: `unsafe { std::env::set_var("ROKO_HIGH_CONTRAST", "1") };`
- Line 2229: `unsafe { std::env::set_var("ROKO_REDUCED_MOTION", "1") };`

## Exact Changes

1. In `util.rs`, remove the `unsafe { std::env::set_var("ROKO_PROVIDER", p) }` call.
2. Add a `provider_override: Option<String>` field to the relevant config/context struct
   that is threaded through dispatch. This could be on `RunConfig`, `DispatchContext`, or
   a new `CliOverrides` struct.
3. In `resolve_effective_model()` in `model_selection.rs`, add a parameter for provider
   override (or check the config struct) before falling back to env var detection.
4. For the `main.rs` accessibility env vars (HIGH_CONTRAST, REDUCED_MOTION): these are
   set before the tokio runtime starts. If they are set in `main()` before `#[tokio::main]`,
   they are safe. If set after runtime start, move them before runtime init or use a config
   field instead.

## Design Guidance

Use a `CliOverrides` struct that carries CLI-level overrides through the call chain:
```rust
pub struct CliOverrides {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
}
```
Thread this through `cmd_run()`, `cmd_chat()`, etc. as a parameter. This is more
explicit and safe than environment variables.

## Write Scope

- `crates/roko-cli/src/commands/util.rs`
- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-cli/src/main.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -rn 'unsafe.*set_var\|set_var.*unsafe' crates/roko-cli/src/` returns zero matches
- [ ] `roko run --provider cerebras "hello"` uses Cerebras without env var mutation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn 'unsafe.*set_var\|set_var.*unsafe' crates/roko-cli/src/` returns zero matches
- `roko run --provider cerebras "hello"` uses Cerebras without env var mutation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
