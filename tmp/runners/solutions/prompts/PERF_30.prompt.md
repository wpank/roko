# PERF_30: Add `--output json` to `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-30`](../ISSUE-TRACKER.md#perf-30)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.30
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Structured JSON output for automation and HAL wrapper.

## Exact Changes

1. Add `--output <FORMAT>` to `run` subcommand: `text` (default) or `json`
2. Define:
   ```rust
   #[derive(Serialize)]
   struct RunOutputJson {
       success: bool,
       model: String,
       cost_usd: f64,
       total_tokens: u64,
       input_tokens: u64,
       output_tokens: u64,
       duration_ms: u64,
       gate_results: Vec<GateResultJson>,
       files_changed: Vec<String>,
       error: Option<String>,
   }
   ```
3. At end of `run_once()`, if `--output json`: serialize and print to stdout
4. Suppress non-JSON output (progress bars, status) when `--output json`

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/main.rs`

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

- [ ] `roko run --output json "echo hello" | jq .success` outputs `true`/`false`
- [ ] JSON includes all fields with correct types
- [ ] `--output text` behavior unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run --output json "echo hello" | jq .success` outputs `true`/`false`
- JSON includes all fields with correct types
- `--output text` behavior unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
