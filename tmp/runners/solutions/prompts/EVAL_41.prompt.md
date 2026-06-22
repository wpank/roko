# EVAL_41: `roko eval` CLI command family

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-41`](../ISSUE-TRACKER.md#eval-41)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.41
- Priority: **P1**
- Effort: 8 hours
- Depends on: `EVAL_04` (source 5.4), `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_41 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Commands: `roko eval run <path>`, `roko eval list`, `roko eval show <profile>`, `roko eval history`, `roko eval trace <id>`, `roko eval compare <id1> <id2>`, `roko eval calibrate`.

## Exact Changes

1. Define `EvalCommand` enum with clap subcommands.
2. Implement `roko eval list`: reads profiles from built-in + `.roko/eval/profiles/*.toml`.
3. Implement `roko eval history`: reads from `TraceStore::recent(limit)`, renders table.
4. Implement `roko eval trace <id>`: renders full trace detail.
5. Implement `roko eval run <path>`: constructs `ArtifactRef`, resolves profile, runs `EvalService::evaluate()`, prints results.
6. Register in main CLI dispatch.

## Write Scope

- `crates/roko-cli/src/commands/mod.rs`
- `crates/roko-cli/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Smoke test: `roko eval list` parses and runs (even with no profiles)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_41 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Smoke test: `roko eval list` parses and runs (even with no profiles)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_41 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
