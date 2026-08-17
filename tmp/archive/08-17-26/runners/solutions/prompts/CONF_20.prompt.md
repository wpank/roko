# CONF_20: Audit and Document `legacy-orchestrate` Feature Flag

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-20`](../ISSUE-TRACKER.md#conf-20)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.20
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `legacy-orchestrate` feature flag is ON by default (`Cargo.toml` line 16:
`default = ["legacy-orchestrate"]`). It gates code in 7 files:
`run.rs`, `dispatch_direct.rs`, `chat_inline.rs`, `lib.rs`, `auth_detect.rs`,
`unified.rs`, and the massive `orchestrate.rs` (~21K lines).

It is unclear what the migration path is, what the flag controls, and whether
disabling it breaks anything critical.

## Exact Changes

1. Document the current default state and what the flag controls.
2. Add `// DEPRECATED: legacy-orchestrate` comment blocks at every gated section.
3. Create a tracking list of all gated code blocks (file, line range, what it does).
4. For code that is the only implementation of a needed feature: plan extraction to
   an ungated module. For purely dead code: mark with `#[deprecated]`.
5. Append the tracking list to `.roko/GAPS.md`.

## Write Scope

- `crates/roko-cli/Cargo.toml`

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

- [ ] Every `#[cfg(feature = "legacy-orchestrate")]` section has adjacent documentation.
- [ ] A tracking list exists in `.roko/GAPS.md` listing all gated blocks.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every `#[cfg(feature = "legacy-orchestrate")]` section has adjacent documentation.
- A tracking list exists in `.roko/GAPS.md` listing all gated blocks.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
