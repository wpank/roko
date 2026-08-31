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

## Current status

- [x] Deterministic fixed-SHA cold/warm runner and representative fixture manifest.
- [x] Evidence bundles, machine-readable raw rows, nearest-rank p50/p95, and FAST comparisons.
- [x] Offline Cargo behavior, explicit provider authorization/cost budget, projected disk reserve,
  bounded targets, and ownership-checked disposable cold caches.
- [x] Historical pictured-run facts are retained without pretending they are a complete sample.
- [ ] Run five cold and five warm repetitions for every promoted fixture/lane.
- [ ] Import five manual Codex and Claude samples per selected fixture/cache.
- [ ] Record escaped regressions and full-CI pass-rate baseline before FAST promotion.
