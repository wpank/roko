# PERF_29: Create HAL Agent Wrapper

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-29`](../ISSUE-TRACKER.md#perf-29)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.29
- Priority: **??**
- Effort: ?
- Depends on: `PERF_30` (source 10.30)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Python wrapper exposing roko as HAL-compatible agent for standardized
benchmark evaluation (SWE-bench, etc.).

## Exact Changes

1. Create `hal/roko_agent/main.py` with `run(task, **kwargs)` function per HAL's
   agent protocol
2. Wrapper logic:
   - Accept task dict: `instance_id`, `prompt`/`problem_statement`, `repo`,
     `base_commit`, `hints`
   - Clone/checkout task repo into temp dir
   - Run `roko init` + `roko run --model <model> --output json "<prompt>"`
   - Capture `git diff HEAD` as `model_patch`
   - Return: `model_patch`, `cost`, `tokens`, `duration_s`, `model`, `exit_code`
3. Create `hal/roko_agent/requirements.txt` (stdlib only, no deps)
4. Support kwargs: `model_name`, `workflow`, `gates`, `timeout`, `roko_binary`

## Write Scope

- `hal/roko_agent/main.py`
- `hal/roko_agent/requirements.txt`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `hal-eval` can invoke the wrapper and get a valid result dict
- [ ] `model_patch` is a valid unified diff
- [ ] Timeout respected (process killed after limit)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `hal-eval` can invoke the wrapper and get a valid result dict
- `model_patch` is a valid unified diff
- Timeout respected (process killed after limit)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
