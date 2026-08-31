# Roko development benchmark

This is the executable companion to the development speed scorecard. It compares the current Roko
lane with opt-in FAST at one immutable Git commit and supports imported manual Codex/Claude samples.
It does not promote FAST or claim a performance win until the representative matrix has actually
been run and full-CI/escaped-regression evidence has been added.

## Inspect before spending or executing

```bash
python3 scripts/dev_benchmark.py list
python3 scripts/dev_benchmark.py run --dry-run --base <commit>
```

The stock command lanes use providers, so execution fails closed without both `--allow-network`
and an explicit `--max-cost-usd` ceiling. Commands are argv arrays rather than shell strings. The
runner also caps repetitions, per-command deadline, total deadline envelope, run count, free disk,
session size, and individual cache size.

The final integration smoke listed four configured lanes and dry-planned 140 measured runs without
executing providers or builds; admission correctly required explicit network authorization and a
cost ceiling. A synthetic two-session history fixture with three samples per session and p50
moving from 100 ms to 200 ms exited 1, marked the comparison regressed, emitted two alerts, and
wrote both JSON and Markdown. These results verify orchestration/alert mechanics only.

## Run a narrow trial, then the matrix

Start with one fixture and one repetition:

```bash
python3 scripts/dev_benchmark.py run \
  --base <commit> \
  --roko-bin /path/to/roko-built-from-that-commit \
  --binary-base <commit> \
  --fixture enum-config \
  --repetitions 1 \
  --allow-network \
  --max-cost-usd 3
```

After inspecting that session, run the selected five-by-five matrix:

```bash
python3 scripts/dev_benchmark.py run \
  --base <commit> \
  --roko-bin /path/to/roko-built-from-that-commit \
  --binary-base <commit> \
  --repetitions 5 \
  --allow-network \
  --max-cost-usd <reviewed-ceiling>
```

Selection flags (`--lane`, `--fixture`, and `--cache`) are repeatable. The process is deliberately
serial so Cargo lock contention and provider overlap do not contaminate samples.

The runner never guesses that the primary worktree's debug binary matches `--base`. Stock Roko
lanes require an explicit executable and an operator-attested `--binary-base` resolving to the same
commit. Its SHA-256 and attestation are recorded. `--allow-unverified-binary` exists only as a
visible escape hatch and makes the session unsuitable for an exact-SHA promotion claim.

## Cache semantics

- Cold never invokes `cargo clean` and never deletes a shared target. Every cold sample receives a
  unique, initially absent `CARGO_TARGET_DIR` below its private benchmark session. After timing and
  evidence are finalized and the command process group is proven absent, that exact disposable
  directory is removed by default. `--keep-targets` is the explicit diagnostic override.
- Warm gets one stable bounded target per lane. Each fixture's deterministic Cargo warmup is
  evidence-captured and excluded from percentiles; measured repetitions then reuse that lane target.
- `CARGO_NET_OFFLINE=true` prevents Cargo from downloading dependencies during either strategy.
- Settled evidence-bearing benchmark-owned Git worktrees are removed with an exact
  registered-worktree and process-absence check, including failed samples whose raw patch evidence
  is already bundled. Unsettled paths are retained; `--keep-worktrees` retains all of them.
- Warm targets and all raw evidence are retained. Cold cleanup requires a matching session owner
  marker, canonical exact-child path, real non-symlink directory, non-overlap with the repository
  target, and confirmed process-group settlement. There is no broad target cleanup command here.

## Evidence and output

Each session is private under `.roko/benchmarks/<session-id>/` by default and contains:

```text
session.json             fixed SHA, binary hash, matrix, and resource/cost limits
admission.jsonl          per-run free-space and bounded target/session sizing
runs.jsonl               one identity/terminal/metrics row per warmup or sample
samples/*/evidence/*     complete run-evidence bundles
worktrees.jsonl          exact creation and safe-cleanup decisions
targets.jsonl            measured target sizes and exact cold disposal decisions
scorecard.json           machine-readable p50/p95, raw rows, and comparisons
SCORECARD.md             compact human view
```

Failures and timeouts stay in the distribution at their observed wall/deadline duration. Missing
fields remain explicit. Warmups never enter measured percentiles. Rebuild summaries without
executing a workload:

```bash
python3 scripts/dev_benchmark.py summarize .roko/benchmarks/<session-id>
```

Manual Codex/Claude lanes are baseline/import-only in the stock manifest so the runner never
guesses an installed CLI or silently grants it network access. Copy the manifest, add an explicit
argv adapter, and select it with `--manifest`; or import rows shaped like
`manual-baselines.json` through `--baseline`.

## Historical dashboard and regression alerts

Refresh the deterministic dashboard after a benchmark session:

```bash
./dev.sh benchmark history
```

This scans only direct, real session directories below `.roko/benchmarks`, keeps the newest 100,
and writes `.roko/benchmarks/history.json` plus `.roko/benchmarks/HISTORY.md`. The JSON includes
session rollups, per-lane/fixture/cache series, the exact policy, every missing/unreadable-session
issue, and the latest comparison. `scorecard.json` raw measured rows are preferred; a bounded
`runs.jsonl` fallback is used when the scorecard is absent or unusable. Imported historical rows
and warmups are not re-counted as session samples.

By default the newest selected session is compared with the immediately previous session. Pin an
older reviewed baseline with `--baseline-session <session-id>` or select a candidate with
`--candidate-session <session-id>`. The default policy requires three samples in both groups and
alerts above a 15% p50 regression, 20% p95 regression, 5 percentage-point non-success or timeout
increase, or 5-point validated-rate drop. Every threshold has an explicit CLI flag. A breached
threshold exits 1 after writing both dashboards, which makes the command directly usable in CI;
`--report-only` suppresses that exit, while `--fail-on-inconclusive` also fails missing/undersampled
comparisons.

The scan fails closed rather than publish a filesystem-order-dependent partial history when its
root-entry, total-byte, or deadline bound is exceeded. Defaults are 2,000 root entries, 100
sessions, 2,000 rows and 256 groups per session, 32 MiB per artifact, 256 MiB total, and 10 seconds.
Increase a named cap deliberately when retained history outgrows it. Failed and timed-out rows stay
in the non-success distribution, missing validity remains counted and visible, and absent latency
is reported as inconclusive rather than zero or silently discarded.

## Current status

- [x] Deterministic fixed-SHA cold/warm runner and representative fixture manifest.
- [x] Evidence bundles, machine-readable raw rows, nearest-rank p50/p95, and FAST comparisons.
- [x] Offline Cargo behavior, explicit provider authorization/cost budget, projected disk reserve,
  bounded targets, and ownership-checked disposable cold caches.
- [x] Historical pictured-run facts are retained without pretending they are a complete sample.
- [x] Bounded deterministic historical JSON/Markdown dashboard with previous/fixed-baseline
  regression alerts and CI exit semantics.
- [x] Smoke the four-lane manifest/no-execution 140-run matrix plan and the history regression exit
  path with a deterministic synthetic two-session fixture.
- [ ] Run five cold and five warm repetitions for every promoted fixture/lane.
- [ ] Import five manual Codex and Claude samples per selected fixture/cache.
- [ ] Record escaped regressions and full-CI pass-rate baseline before FAST promotion.
