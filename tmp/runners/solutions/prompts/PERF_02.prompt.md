# PERF_02: Create Criterion Benchmark Harness

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-02`](../ISSUE-TRACKER.md#perf-02)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.2
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Criterion benchmarks for the five heaviest non-inference functions:
config load, learning runtime open, prompt assembly, substrate write, feedback
flush.

## Exact Changes

1. Add `criterion = { version = "0.5", features = ["html_reports"] }` to
   `[dev-dependencies]` in `crates/roko-cli/Cargo.toml`
2. Add `[[bench]] name = "runtime_overhead" harness = false` entry
3. Create `benches/runtime_overhead.rs` with benchmark groups:
   - `bench_config_load`: time `crate::config::load_layered()` on workspace
     `roko.toml`
   - `bench_learning_open`: time `LearningRuntime::open_under()` with existing
     `.roko/learn/` state (create temp dir with fixture data)
   - `bench_prompt_assembly`: time `PromptAssemblyService::assemble()` with a
     fixture prompt and workspace
   - `bench_substrate_write_10`: time 10 sequential `FileSubstrate::put()` calls
   - `bench_jsonl_flush_30`: time 30 `JsonlLogger::write_event()` calls
4. Each group reports mean, median, std deviation via criterion defaults

## Write Scope

- `crates/roko-cli/benches/runtime_overhead.rs`
- `crates/roko-cli/Cargo.toml`

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

- [ ] `cargo bench --bench runtime_overhead -p roko-cli` runs and produces timing
- [ ] Criterion's HTML report generates at `target/criterion/`
- [ ] Comparison with prior runs via `cargo bench` regression detection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Criterion's HTML report generates at `target/criterion/`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
