# STAB_10: Fix `roko init` emitting wrong gate format

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-10`](../ISSUE-TRACKER.md#stab-10)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.10
- Priority: **P0**
- Effort: 1 hour
- Depends on: `STAB_09` (source 1.09)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`append_shell_gate()` at line 129 writes:
```rust
out.push_str("\n[[gate]]\n");
out.push_str("kind = \"shell\"\n");
```
This writes the `[[gate]]` array format. The runtime expects `[gates]` table format.

## Exact Changes

1. Replace `append_shell_gate()` with a function that writes `[gates]` format:
   ```toml
   [gates]
   enabled = ["compile", "clippy", "test"]

   [[gates.shell]]
   program = "cargo"
   args = ["check", "--workspace"]
   timeout_ms = 120000
   ```
2. For the "no profile" case (line 121), update the comment to reference `[gates]`:
   ```
   # Add [gates] section to configure validation gates.
   ```
3. Update init tests to verify the new format.
4. Verify that `RokoConfig::from_toml()` can parse the generated output.

## Design Guidance

The init template should generate the simplest valid config. For most Rust projects:
```toml
[gates]
enabled = ["compile", "clippy", "test"]
```
Shell gates can be added as a commented-out example.

## Write Scope

- `crates/roko-cli/src/commands/init.rs`

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

- [ ] `roko init --profile rust` generates `[gates]` format, not `[[gate]]`
- [ ] Generated `roko.toml` passes `RokoConfig::from_toml()`
- [ ] `roko plan run` respects gates from init-generated config

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko init --profile rust` generates `[gates]` format, not `[[gate]]`
- Generated `roko.toml` passes `RokoConfig::from_toml()`
- `roko plan run` respects gates from init-generated config
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
