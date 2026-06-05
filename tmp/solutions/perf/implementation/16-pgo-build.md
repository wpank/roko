# 16 — Profile-Guided Optimization Release Build

> Bottleneck: cold compiler heuristics in the release build leave 5-10
> % of CPU time on the floor (branch mispredicts, suboptimal inlining,
> cache-unfriendly layout).
>
> Target savings: 5-10 % overall on the non-IO portions of every
> command.
> Effort: ≈4 h. Risk: low (purely a build pipeline change).

---

## Goal & success criteria

After this change:

1. The release Docker image (and homebrew formula, if applicable) is
   built with `-Cprofile-use=...` after a representative training pass.
2. CI has a `pgo` job that runs nightly and uploads the merged
   `.profdata` artifact.
3. The training corpus is reproducible: a script runs a curated set of
   `roko` commands against fixture workdirs.

Done when:

- Two release binaries exist on disk: `target/release/roko` (LTO+CGU=1)
  and `target/release-pgo/roko` (PGO instrumented + merged data).
- `cargo bench --bench cli_overhead` shows ≥5 % wall-time improvement
  for the `--gates none` baseline on the PGO binary.
- The CI workflow `.github/workflows/release.yml` builds the PGO
  binary as the canonical release artifact.

---

## Background

- Source: `OPTIMIZATION-PLAYBOOK.md` §14.
- Rust supports PGO via `-Cprofile-generate=<dir>` (instrumented
  build) and `-Cprofile-use=<merged.profdata>` (optimised build).
- LLVM's `llvm-profdata` tool merges the per-process raw `.profraw`
  files into a single `.profdata` consumed by the second build.
- Roko is mostly IO-bound (network + subprocess), so PGO will not
  produce miracles. The win is in the hot non-IO bytes: TOML parsing,
  JSON serialization, prompt assembly, gate dispatch, event bus.

---

## Files to read first

| File | Why |
|---|---|
| `.github/workflows/release.yml` (or whichever file builds the release) | Where to add the PGO step. |
| `Cargo.toml` (workspace root) | `[profile.release]` settings; ensure compatibility with PGO (no `panic = "abort"` if you want unwinds). |
| `Dockerfile` | If a Docker release exists, add the PGO toolchain there too. |
| Any existing `cargo bench` setup | Use the same harness for PGO before/after measurement. |

---

## Code-level plan

### Step 1 — Add a training script

New file: `scripts/pgo-train.sh`.

```bash
#!/usr/bin/env bash
# Run a representative workload against an instrumented `roko` binary
# to gather profile data for PGO.
#
# Usage: PGO_DATA=/tmp/pgo-data scripts/pgo-train.sh
set -euo pipefail

PGO_DATA="${PGO_DATA:-/tmp/pgo-data}"
ROKO="${ROKO:-./target/release/roko}"
FIXTURE="${FIXTURE:-./fixtures/pgo-workdir}"

mkdir -p "$PGO_DATA"
echo "Training PGO into $PGO_DATA using $ROKO"

# 1. Cheap commands (config, version, plan validate) — exercise startup.
"$ROKO" config show > /dev/null
"$ROKO" --version > /dev/null
"$ROKO" plan validate "$FIXTURE/plans" > /dev/null || true

# 2. roko run with --gates none (no network, no provider).
#    Use the `mock` provider so we don't need API keys.
"$ROKO" run --provider mock --workflow-template express --gates none \
  "Reply with hello" > /dev/null

# 3. roko bench demo — exercises the full dispatch path.
"$ROKO" bench demo --simulate > /dev/null

# 4. Multiple runs to amortise variance and exercise warm paths.
for i in $(seq 1 20); do
    "$ROKO" run --provider mock --workflow-template express --gates none \
      "iteration $i" > /dev/null
done

echo "Profile data collected:"
du -sh "$PGO_DATA"
```

The fixture workdir at `fixtures/pgo-workdir/` should contain:
- A minimal `roko.toml` configuring the `mock` provider.
- A small `plans/test/` directory with 1-2 simple plans.
- A trivial Rust workspace under `src/` so prompt-assembly /
  conventions detection do real work.

### Step 2 — Two-stage build script

New file: `scripts/pgo-build.sh`.

```bash
#!/usr/bin/env bash
set -euo pipefail

PGO_DATA="${PGO_DATA:-/tmp/pgo-data}"
PROFDATA="$PGO_DATA/merged.profdata"

# 1. Instrumented build.
echo "==> Stage 1: instrumented build"
RUSTFLAGS="-Cprofile-generate=$PGO_DATA" cargo build --release -p roko-cli

# 2. Run training.
echo "==> Stage 2: training"
ROKO=./target/release/roko ./scripts/pgo-train.sh

# 3. Merge profile data.
echo "==> Stage 3: merging profile data"
LLVM_PROFDATA="${LLVM_PROFDATA:-llvm-profdata}"
"$LLVM_PROFDATA" merge -o "$PROFDATA" "$PGO_DATA"

# 4. Optimised build.
echo "==> Stage 4: optimised build"
RUSTFLAGS="-Cprofile-use=$PROFDATA -Cllvm-args=-pgo-warn-missing-function" \
    cargo build --release -p roko-cli --target-dir target-pgo

echo "==> Done. Binary at target-pgo/release/roko"
```

The script depends on `llvm-profdata` being on PATH. On macOS Homebrew
LLVM, it's at `/opt/homebrew/opt/llvm/bin/llvm-profdata`. On Ubuntu,
install via `apt install llvm`. Document this in the script header.

### Step 3 — CI integration

Add a job in `.github/workflows/release.yml`:

```yaml
jobs:
  pgo-build:
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - name: Install llvm-profdata
        run: |
          rustup component add llvm-tools-preview
          # rustup ships llvm-profdata under the sysroot; locate it.
          PROFDATA=$(find $(rustc --print sysroot) -name "llvm-profdata" -type f | head -1)
          echo "LLVM_PROFDATA=$PROFDATA" >> $GITHUB_ENV
          echo "$(dirname $PROFDATA)" >> $GITHUB_PATH
      - name: PGO build
        run: ./scripts/pgo-build.sh
        env:
          PGO_DATA: ${{ runner.temp }}/pgo-data
      - name: Upload PGO binary
        uses: actions/upload-artifact@v4
        with:
          name: roko-pgo-${{ github.sha }}
          path: target-pgo/release/roko
```

> **Anti-pattern.** Do not run PGO on `pull_request` events. The
> instrumented build + training takes 5-10 minutes; running it on every
> PR is wasteful. PGO belongs on `push: branches: [main]` or a manual
> trigger, with the artifact promoted to release.

### Step 4 — `cargo bench` harness for measuring PGO benefit

If no harness exists, add one at `crates/roko-cli/benches/cli_overhead.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use std::process::Command;

fn bench_cli_overhead(c: &mut Criterion) {
    c.bench_function("config_show", |b| {
        b.iter(|| {
            Command::new(env!("CARGO_BIN_EXE_roko"))
                .args(["config", "show"])
                .output()
                .unwrap()
        });
    });

    c.bench_function("plan_validate", |b| {
        b.iter(|| {
            Command::new(env!("CARGO_BIN_EXE_roko"))
                .args(["plan", "validate", "fixtures/pgo-workdir/plans"])
                .output()
                .unwrap()
        });
    });
}

criterion_group!(benches, bench_cli_overhead);
criterion_main!(benches);
```

Add `criterion = "0.5"` as a dev-dep.

Run with:

```bash
cargo bench -p roko-cli --bench cli_overhead
```

Compare results between `target/release/roko` and
`target-pgo/release/roko`.

---

## Step-by-step execution

1. `git checkout -b perf/16-pgo-build`.
2. Create `fixtures/pgo-workdir/` (Step 1 prereq).
3. Add `scripts/pgo-train.sh`, `scripts/pgo-build.sh`. `chmod +x`.
4. Local test: run `scripts/pgo-build.sh`. Verify output binary exists
   and runs.
5. Add CI job (Step 3).
6. Add `cargo bench` harness (Step 4) if not present.
7. Run benches before/after and record results in PR.
8. PR `build(release): add PGO build pipeline (5-10% improvement)`.

---

## Anti-patterns / things NOT to do

- **Do NOT enable PGO in `[profile.release]`** in `Cargo.toml`. It
  would force every contributor's local `cargo build --release` to
  collect profile data with no merged `.profdata` available, producing
  unusable binaries. PGO is a release-pipeline concern, not a build
  default.
- **Do NOT train against synthetic micro-benchmarks** (`cargo bench`
  loops). The PGO data must reflect *user workloads*, not allocator
  stress tests. Use realistic command invocations.
- **Do NOT skip the instrumented build's `--release` flag.** Profile
  data from a debug build is meaningless to the optimiser.
- **Do NOT ship the instrumented binary**. It writes `.profraw` files
  on every run. Users will fill their disks.
- **Do NOT mix `panic = "abort"` and PGO** without testing — some
  combinations have known edge cases. Roko's release profile uses
  `panic = "unwind"` (default); leave it.
- **Do NOT spend more than 4 h on PGO** before measuring. If the
  measured improvement is <3 %, skip this plan; the binary maintenance
  cost (CI tax, troubleshooting profdata corruption) is not worth a
  marginal win.
- **Do NOT use sample-based PGO (`-Cllvm-args=-fprofile-sample-use`)**
  unless you have a reason. Instrumentation-based PGO is simpler and
  produces better data for our workload size.
- **Do NOT enable `-Ccodegen-units=1`** for the instrumented build.
  Slower compilation, no benefit during data collection. Keep it for
  the final optimised build.

---

## Test plan

| Level | Test | Where |
|---|---|---|
| Manual | `scripts/pgo-build.sh` succeeds end-to-end | local + CI |
| Bench | `target/release/roko` vs `target-pgo/release/roko` shows ≥5 % improvement on `config_show` and `plan_validate` | criterion output |
| CI | PGO artifact uploaded on `main` branch push | GitHub Actions |

---

## Rollback plan

- The PGO binary is a separate artifact; the standard release binary
  remains unchanged unless the workflow promotes PGO as canonical.
- To disable: set the workflow job condition to `if: false` or remove
  the upload step. Existing release artifacts continue to ship.

---

## Status check (acceptance)

- [ ] `scripts/pgo-train.sh` and `scripts/pgo-build.sh` exist and run
      locally.
- [ ] Fixture workdir under `fixtures/pgo-workdir/` exists.
- [ ] `cargo bench` harness shows ≥5 % wall-time improvement.
- [ ] CI job `pgo-build` exists on `main` push.
- [ ] PR description includes before/after benchmark numbers.
