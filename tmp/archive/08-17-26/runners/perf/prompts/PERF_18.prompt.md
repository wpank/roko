# PERF_18: PGO release build pipeline

## Task

Add **instrumented build → training workload → profdata merge → PGO
optimized build** scripts, a **minimal fixture workdir** for training,
a **Criterion** bench comparing hot CLI paths, and a **`pgo-build` CI
job** that uploads the PGO binary as an artifact. Do **not** turn PGO on
for every developer’s default `cargo build --release`.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_18](../ISSUE-TRACKER.md#perf_18)
- Plan: `tmp/solutions/perf/implementation/16-pgo-build.md`
- Performance contract: **C-17**
- Priority: P4
- Effort: ≈4 h
- Depends on: none
- Wave: 3

## Problem

Release binaries are built without profile feedback. LLVM PGO can
squeeze **5–10 %** on CPU-heavy paths (TOML/JSON, dispatch setup) even
when the product is IO-dominated.

## Exact Changes

### Step 1 — `scripts/pgo-train.sh`

- `#!/usr/bin/env bash`, `set -euo pipefail`.
- Env: `PGO_DATA` (profile output dir), `ROKO` (path to instrumented
  binary, default `./target/release/roko`), `FIXTURE` (default
  `./fixtures/pgo-workdir`).
- Run the commands listed in the plan §Step 1 (`config show`, `--version`,
  `plan validate`, `run` with `--provider mock`, `bench demo --simulate`,
  loop 20× `run` mock) — adjust flags to match **actual** CLI in this
  repo (read `roko run --help` / existing tests).
- Header comment: document `llvm-profdata` location on macOS Homebrew vs
  Ubuntu `apt`.

### Step 2 — `scripts/pgo-build.sh`

Stages:

1. `RUSTFLAGS="-Cprofile-generate=$PGO_DATA" cargo build --release -p roko-cli`
2. Run `pgo-train.sh` with `ROKO=./target/release/roko`.
3. **Merge** raw profiles: use `llvm-profdata merge -o "$PROFDATA"`
   with explicit **`$PGO_DATA/*.profraw`** globs (the plan’s bare
   `"$PGO_DATA"` directory form is **wrong** for `llvm-profdata merge` —
   fix it).
4. Final build: `RUSTFLAGS="-Cprofile-use=$PROFDATA -Cllvm-args=-pgo-warn-missing-function" cargo build --release -p roko-cli --target-dir target-pgo` (or equivalent `--target-dir` as in plan).
5. Echo final binary path `target-pgo/release/roko`.

`chmod +x` both scripts (git mode 100755).

### Step 3 — `fixtures/pgo-workdir/`

- Minimal `roko.toml` with **mock** provider (copy patterns from existing
  fixtures/tests).
- `plans/test/plan.md` (or `plans/test/` layout expected by `plan validate`).
- Tiny `src/` tree (e.g. `src/lib.rs` or `main.rs`) so prompt/convention
  code exercises real paths.

### Step 4 — Criterion bench `crates/roko-cli/benches/cli_overhead.rs`

- Bench functions: at minimum `config_show` and `plan_validate` spawning
  `CARGO_BIN_EXE_roko` (see plan §Step 4).
- Add `criterion` as **dev-dependency** in `crates/roko-cli/Cargo.toml`
  + `[[bench]]` entry `name = "cli_overhead"`, `harness = false` if required.

### Step 5 — `.github/workflows/release.yml`

- Add job `pgo-build` per plan §Step 3:
  - `dtolnay/rust-toolchain@stable` with `components: llvm-tools-preview`
    OR install system `llvm-profdata` — ensure `LLVM_PROFDATA` env points
    to a working binary.
  - Run `./scripts/pgo-build.sh` with `PGO_DATA` under `${{ runner.temp }}/pgo-data`.
  - **Trigger:** only on `push` to `main` (and optionally
    `workflow_dispatch`). **Do NOT** add `pull_request` for this job
    (anti-pattern in plan).
  - Upload artifact `roko-pgo-${{ github.sha }}` with
    `target-pgo/release/roko`.

### Step 6 — Dockerfile (only if needed)

If `Dockerfile` in this repo produces release images, add a short comment
or stage hook pointing operators to `scripts/pgo-build.sh` — do **not**
bloat Docker build time on every PR unless already agreed in code review.

## Write Scope

- `scripts/pgo-train.sh` (**new**)
- `scripts/pgo-build.sh` (**new**)
- `fixtures/pgo-workdir/**` (**new**, minimal tree)
- `crates/roko-cli/benches/cli_overhead.rs` (**new**)
- `crates/roko-cli/Cargo.toml` (criterion + bench table)
- `.github/workflows/release.yml`
- `Dockerfile` (optional comment only)

## Read-Only Context

- `Cargo.toml` (workspace profiles — do not set `profile-release` PGO by default)
- `tmp/solutions/perf/implementation/16-pgo-build.md`

## Acceptance Criteria

- [ ] Both scripts exist, executable, and `pgo-build.sh` completes locally when llvm tools exist.
- [ ] Fixture workdir satisfies training script paths.
- [ ] `cargo bench -p roko-cli --bench cli_overhead` runs (post-merge verification).
- [ ] CI job `pgo-build` uploads `roko-pgo-${{ github.sha }}`.
- [ ] PGO job **not** on `pull_request` only.
- [ ] PR description should cite before/after bench numbers (human step).
- [ ] Commit message trailer: `tracker: PERF_18 done <sha>`.

## Verify

```bash
rg -n 'pgo-build|profile-generate|profile-use' scripts .github/workflows/release.yml crates/roko-cli/Cargo.toml
# Post-merge (allowed outside batch):
# LLVM_PROFDATA=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-profdata
# ./scripts/pgo-build.sh
```

## Do NOT

- Do NOT add `-Cprofile-generate` to `[profile.release]` in root `Cargo.toml`.
- Do NOT ship the instrumented binary as the default release artifact
  without merging profdata.
- Do NOT train only on `cargo bench` micro-loops; the training script must
  run real `roko` subcommands.
- Do NOT enable `-Ccodegen-units=1` on the **instrumented** build (plan).
- Do NOT compile or run tests **during** the agent batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_18 done <commit-sha>
```
