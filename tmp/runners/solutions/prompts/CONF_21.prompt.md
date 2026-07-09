# CONF_21: Enable `hdc` Feature by Default for `roko-neuro`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-21`](../ISSUE-TRACKER.md#conf-21)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.21
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko-neuro/Cargo.toml` has `default = []` at line 16. The `hdc` feature (line 17:
`hdc = ["dep:roko-primitives"]`) is required for anti-knowledge gating and HDC-based
similarity scoring in `KnowledgeStore`. Without it, quality-control mechanisms are
inactive in default builds.

## Exact Changes

1. Check whether `roko-cli`'s dependency on `roko-neuro` enables `hdc`:
   look for `features = ["hdc"]` in roko-cli's Cargo.toml.
2. If not enabled: either add `hdc` to `roko-neuro`'s default features
   (`default = ["hdc"]`), or enable it in roko-cli's dependency declaration.
3. If `roko-primitives` has heavy dependencies, keep `hdc` optional but ensure
   the roko-cli binary enables it.

## Write Scope

- `crates/roko-neuro/Cargo.toml`
- `Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Building `roko-cli` with default features includes HDC fingerprinting.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Building `roko-cli` with default features includes HDC fingerprinting.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
