# TEST_05: Learning subsystem integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-05`](../ISSUE-TRACKER.md#test-05)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.5
- Priority: **P0**
- Effort: 5 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Key types to test:
- `EpisodeLogger` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs` (line 911) -- append-only JSONL
- `CascadeRouter` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` (line 82) -- bandit-based model routing
- `SectionEffectivenessRegistry` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/section_effect.rs` (line 114)
- `PlaybookStore` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs` (line 652)
- `AgentEfficiencyEvent` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` (line 80)
- `DriftDetector` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/drift.rs` (line 89)

Existing tests in `learning_loop.rs` (4 tests) and `cascade_router_integration.rs` cover basic paths but not concurrent access, field integrity under volume, or persistence roundtrips.

## Exact Changes

1. Test `EpisodeLogger`: write 100 episodes, read back, verify ordering and field integrity (all fields non-default)
2. Test `EpisodeLogger`: concurrent appends from 5 tokio tasks (20 episodes each), verify total count = 100
3. Test `CascadeRouter`: `load_or_new`, observe 50 outcomes across 3 models, save to disk, reload from same file, verify observation counts match
4. Test `CascadeRouter`: routing decisions shift after observing 20 successes for model A and 20 failures for model B
5. Test `SectionEffectivenessRegistry`: record positive/negative signals for 10 sections, verify weights shift in expected direction
6. Test `SectionEffectivenessRegistry`: persist and reload, verify weights and counts match
7. Test `PlaybookStore`: write 5 playbooks with different roles/categories, query by role, verify correct subset returned
8. Test `PlaybookStore`: query by category, verify filtering works
9. Test `AgentEfficiencyEvent`: write 50 events, verify JSONL line count matches
10. Test `DriftDetector`: feed 100 increasing values, verify drift detected
11. Test `DriftDetector`: feed 100 stable values, verify no drift detected

## Write Scope

- `crates/roko-learn/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 11+ new tests, all passing
- [ ] Every learning artifact type has at least one integration test
- [ ] Concurrent access test uses real tokio tasks (not serial simulation)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 11+ new tests, all passing
- Every learning artifact type has at least one integration test
- Concurrent access test uses real tokio tasks (not serial simulation)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
