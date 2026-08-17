# PERF_27: PGO Build Script

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-27`](../ISSUE-TRACKER.md#perf-27)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.27
- Priority: **??**
- Effort: ?
- Depends on: `PERF_02` (source 10.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Script that builds instrumented binary, runs workloads, merges
profiles, rebuilds with PGO data.

## Exact Changes

1. Create `scripts/pgo-build.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   PGO_DIR="${1:-/tmp/roko-pgo-data}"
   rm -rf "$PGO_DIR" && mkdir -p "$PGO_DIR"
   RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --release -p roko-cli
   ./target/release/roko config show 2>/dev/null || true
   ./target/release/roko plan validate plans/ 2>/dev/null || true
   ./target/release/roko status 2>/dev/null || true
   llvm-profdata merge -o "$PGO_DIR/merged.profdata" "$PGO_DIR"
   RUSTFLAGS="-Cprofile-use=$PGO_DIR/merged.profdata" cargo build --release -p roko-cli
   echo "PGO build complete: target/release/roko"
   ```
2. `chmod +x scripts/pgo-build.sh`
3. Document prerequisite: `rustup component add llvm-tools-preview`

## Write Scope

- `scripts/pgo-build.sh`

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

- [ ] `./scripts/pgo-build.sh` completes without error
- [ ] Resulting binary comparable in size to non-PGO release build
- [ ] Benchmark: 5-15% improvement on `bench_config_load` and `bench_prompt_assembly`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `./scripts/pgo-build.sh` completes without error
- Resulting binary comparable in size to non-PGO release build
- Benchmark: 5-15% improvement on `bench_config_load` and `bench_prompt_assembly`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
