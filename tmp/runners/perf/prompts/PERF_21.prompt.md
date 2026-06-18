# PERF_21: Bench compare subcommand + stable result layout

## Task

Add **`roko bench compare`** to diff two serialized bench run artifacts,
fail CI-style on regressions beyond `--threshold`, enforce a
**config-hash sanity check** (`--force` escape hatch), and persist bench
outputs under **`.roko/bench/perf/`** with an **atomic `latest`**
symlink update. Depends on **PERF_20** (trials + consistency types must
exist).

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_21](../ISSUE-TRACKER.md#perf_21)
- Plan: `tmp/solutions/perf/implementation/18-bench-suite-extension.md` (§4–5)
- Performance contract: **C-20**
- Priority: EX
- Depends on: **PERF_20**
- Wave: 2

## Problem

Without a compare tool, CI cannot detect **which task regressed** when
latency or pass-rate shifts. Without a stable on-disk layout, nightly
jobs cannot diff artifacts.

## Exact Changes

### Step 1 — `crates/roko-cli/src/commands/bench_compare.rs` (new)

- `#[derive(Parser)] pub struct BenchCompareArgs`:
  - positional `baseline: PathBuf`, `candidate: PathBuf`
  - `--threshold <f64>` default **20.0** (percent regression on wall-time
    or the metric chosen in code — document in `--help`)
  - `--pareto` flag to print Pareto-style summary (can be stub that reads
    `cost_usd` + pass flag from `BenchRunResult` if full frontier is too
    large for first PR — but preference: implement minimal frontier from
    plan §Step 4).
  - `--force` to proceed when `config_hash` differs.

- `pub async fn run_bench_compare(args: BenchCompareArgs) -> anyhow::Result<()>`
  (async only if file IO uses tokio; sync is fine).

### Step 2 — Deserialize `BenchRunResult`

- Reuse the **same** struct layout `roko bench` / serve writes today; if
  duplicated, consolidate into a small shared type module (`bench_types`
  in `roko-cli` or `roko-core`) — avoid three incompatible JSON shapes.

### Step 3 — Regression logic

- For each task in `candidate.tasks`, find matching `id` in baseline.
- Compute `delta_pct` vs baseline duration (guard divide-by-zero → skip
  or treat as infinite regression per documented rule).
- Collect tasks where `delta_pct > threshold`.
- Print human-readable table to stdout; regressions to stderr.
- Exit code **1** if any regression; **0** if clean.

### Step 4 — Config hash sanity

- If `baseline.config_hash != candidate.config_hash`, print a **warning**
  and **exit non-zero** unless `--force` (plan).

### Step 5 — Persisted layout (writer path in bench runner)

Under `crates/roko-cli/src/bench.rs` (or helper module), after a bench
run completes, optionally write (feature-gated by existing flags or new
`--persist` — **choose minimal invasiveness**):

```text
.roko/bench/perf/YYYYMMDD-HHMMSS/
  suite-results.json
  consistency.json
  pareto.json
  summary.md
```

- Use `roko_fs::atomic::atomic_write_json` (or existing atomic helper) for
  each file.
- Update `latest` symlink: write temp symlink → `rename` (atomic on Unix;
  document Windows limitation if applicable).

### Step 6 — Wire CLI

- `crates/roko-cli/src/commands/mod.rs`: export module + command enum variant.
- `crates/roko-cli/src/main.rs`: dispatch `BenchCompareArgs`.

## Write Scope

- `crates/roko-cli/src/commands/bench_compare.rs` (**new**)
- `crates/roko-cli/src/commands/mod.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/bench.rs` (persist helper + types if needed)

## Read-Only Context

- `crates/roko-fs/src/atomic.rs`
- `tmp/solutions/perf/implementation/18-bench-suite-extension.md`

## Acceptance Criteria

- [ ] `bench_compare.rs` exists with `BenchCompareArgs` + runner.
- [ ] `roko bench compare A B --threshold 20` exits **1** on regression.
- [ ] Config hash mismatch warns and blocks unless `--force`.
- [ ] `.roko/bench/perf/<stamp>/` layout + `atomic_write_json` + atomic `latest`.
- [ ] Test `bench_compare_fails_on_regression` passes (construct minimal JSON fixtures in tempdir).
- [ ] Commit message trailer: `tracker: PERF_21 done <sha>`.

## Verify

```bash
rg -n 'bench compare|BenchCompareArgs|bench/perf' crates/roko-cli/src
./target/release/roko bench compare --help
```

## Do NOT

- Do NOT compare unrelated JSON schemas — unify types with PERF_20 output.
- Do NOT silently ignore `config_hash` mismatch.
- Do NOT add nightly CI workflow in this batch unless already trivially
  composable; plan §Step 7 can be a follow-up if scope explodes (call out
  in PR if deferred).
- Do NOT compile or run tests during the batch (`00-RULES.md`).

## Tracker update

```
tracker: PERF_21 done <commit-sha>
```
