# TEST_21: Performance baseline capture

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-21`](../ISSUE-TRACKER.md#test-21)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.21
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_03` (source 15.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

This task establishes measurable performance baselines for critical paths. All measurements use `std::time::Instant` for wall-clock timing. Thresholds are generous (2-3x expected) to avoid CI flakiness while still catching genuine regressions.

## Exact Changes

1. Test gate pipeline latency: run compile+clippy+test on a minimal `GateTestProject`, assert total time < 30 seconds
2. Test prompt assembly latency: assemble a 9-layer system prompt via `SystemPromptBuilder` with mock data, assert time < 100ms
3. Test episode logger throughput: write 1000 episodes via `EpisodeLogger`, assert time < 1 second
4. Test TOML parsing throughput: parse a 50-task plan TOML (construct programmatically), assert time < 50ms
5. Test config loading time: load a full `roko.toml` with providers and models, assert time < 50ms
6. Test state persistence roundtrip: save/load a 100-task executor state, assert < 100ms
7. Write baseline values to `perf_baselines.json` fixture file; tests print actual measurements

## Design Guidance

Baselines should be stored as a JSON map: `{ "gate_pipeline_ms": 30000, "prompt_assembly_ms": 100, ... }`. Each test reads the baseline, measures, prints actual vs baseline, and asserts within threshold. The `#[ignore]` attribute should NOT be used -- these tests must run in CI to catch regressions early. Thresholds should be generous enough for CI runners.

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] 6+ tests, all passing with generous initial baselines
- [ ] `perf_baselines.json` committed as test fixture
- [ ] Each test prints actual measurement for CI visibility
- [ ] Tests use `std::time::Instant` (not `SystemTime`)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 6+ tests, all passing with generous initial baselines
- `perf_baselines.json` committed as test fixture
- Each test prints actual measurement for CI visibility
- Tests use `std::time::Instant` (not `SystemTime`)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
