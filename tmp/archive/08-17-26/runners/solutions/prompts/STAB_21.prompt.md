# STAB_21: Consolidate 4 stream-json parsing copies

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-21`](../ISSUE-TRACKER.md#stab-21)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.21
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The stream-json parsing logic is duplicated 4 times with inconsistent output formats. All
copies independently implement 4096-byte truncation with char_boundary checks. The canonical
parser `parse_stream_line()` exists in the provider module.

## Exact Changes

1. Identify the canonical `parse_stream_line()` function location.
2. Replace inline parsing in `translate/mod.rs:extract_text()` with calls to the canonical parser.
3. Replace inline parsing in `translate/mod.rs:extract_tool_outputs()` similarly.
4. Replace inline parsing in `chat.rs:extract_clean_text()` similarly.
5. Leave `dispatch_direct.rs` as-is (behind `legacy-orchestrate` feature gate, will be removed).
6. Add tests verifying all replaced paths produce identical output to the canonical parser.

## Design Guidance

The canonical parser should be in `roko-agent` (since it parses agent stream output) and
exported publicly. All consumers in `roko-cli` should import from `roko-agent`. If the
canonical parser is in `roko-cli`, consider moving it to `roko-agent` for the right
dependency direction.

## Write Scope

- `crates/roko-agent/src/translate/mod.rs`
- `crates/roko-cli/src/chat.rs`
- `crates/roko-cli/src/dispatch_direct.rs`

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

- [ ] `grep -rn 'serde_json::from_str.*result' crates/roko-cli/src/chat.rs crates/roko-agent/src/translate/mod.rs` returns zero matches (all delegated)
- [ ] Tests verify identical output from canonical parser and removed inline parsers

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn 'serde_json::from_str.*result' crates/roko-cli/src/chat.rs crates/roko-agent/src/translate/mod.rs` returns zero matches (all delegated)
- Tests verify identical output from canonical parser and removed inline parsers
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
