# XCUT_03: Add ErrorKind Coverage for Missing Subsystems

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-03`](../ISSUE-TRACKER.md#xcut-03)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.3
- Priority: **P7**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ErrorKind` in `crates/roko-core/src/error/mod.rs` (line 337) has 17 variants covering core subsystems: Store, NotFound, BodyEncode, BodyDecode, Rejected, BudgetExceeded, Io, Json, Invalid, Planning, Agent, Verify, Tool, Chain, Config, Transport, User, Timeout, Cancelled, PermissionDenied, RateLimited. Missing discriminants for ACP, MCP, deployment, and daemon errors. These subsystems currently map to `ErrorKind::Internal` (which does not even exist -- they fall through to generic handling), losing subsystem-specific retry policy capability.

The `kind()` method at line 266 is exhaustive over `RokoError` variants and maps each to an `ErrorKind`. The `is_transient()` method at line 382 uses `ErrorKind` to classify whether retry is appropriate.

## Exact Changes

1. Add variants to `ErrorKind`: `Acp`, `Mcp`, `Deploy`, `Daemon`, `Tui`, `Learning`.
2. Map existing `RokoError` variants to the new discriminants in `kind()`.
3. Extend `is_transient()` with classifications: MCP server crashes = transient, deployment auth failures = not transient, ACP session busy = transient.
4. Add doc comments with retry guidance per kind.
5. Update the exhaustive test at line 526 (`example()` function) to cover new kinds.

## Write Scope

- `crates/roko-core/src/error/mod.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All 18 crates' public errors map to a specific `ErrorKind` (not generic)
- [ ] `ErrorKind` implements `Display` with stable string labels suitable for metrics
- [ ] Exhaustive test in `error/mod.rs` covers all new kinds

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All 18 crates' public errors map to a specific `ErrorKind` (not generic)
- `ErrorKind` implements `Display` with stable string labels suitable for metrics
- Exhaustive test in `error/mod.rs` covers all new kinds
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
