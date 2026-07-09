# SAFE_10: Immutable Gate Configs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-10`](../ISSUE-TRACKER.md#safe-10)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.10
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Gate definitions should be loaded once at pipeline start and frozen.
Currently, gates are read from `roko.toml` which agents can modify mid-run.
An agent that writes to `roko.toml` during execution could change gate behavior.

## Exact Changes

1. At the start of `run()` in `event_loop.rs`, load all gate configs from
   `roko.toml` into an `Arc<Vec<GateConfig>>` (immutable after construction)
2. Hash the gate config files at start using BLAKE3
3. Pass the frozen config to `gate_dispatch` instead of re-reading from disk
4. Before each gate execution, verify the config file hash has not changed
5. If changed, log a `tracing::error!` and use the frozen config (do NOT
   pick up the modified version)

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/gate_dispatch.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Modifying `roko.toml` `[[gates]]` section mid-run does NOT affect running gates
- [ ] An agent that writes to `roko.toml` during execution does not change gate behavior
- [ ] Gate config integrity is verified before each gate run
- [ ] Hash mismatch produces a clear error message

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Modifying `roko.toml` `[[gates]]` section mid-run does NOT affect running gates
- An agent that writes to `roko.toml` during execution does not change gate behavior
- Gate config integrity is verified before each gate run
- Hash mismatch produces a clear error message
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
