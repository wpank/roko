# Benchmark scorecard

## Purpose

Speed changes must be measured on representative fixtures and must not increase escaped
regressions. “Feels faster” and a single warm run are insufficient.

## Execution status

The P0 implementation is merged at `a58bdbacb`. Commit `d1b94b139` adds deterministic fixed-SHA
scorecard orchestration in `scripts/dev_benchmark.py`, but the representative matrix has not yet
been run. The runner uses detached worktrees, isolated cold targets, bounded lane-local warm
targets, evidence bundles, and raw/p50/p95 output. It retains failures/timeouts and never
manufactures missing cold/warm or manual evidence. Historical P0 smokes and the coordinator's final
integration checkpoint are mechanics/release evidence, not promotion data. That checkpoint
enumerated four lanes, dry-planned 140 measured runs without executing them, correctly required
network/cost admission, and proved the history alert path with a synthetic three-sample-per-session
100 ms to 200 ms p50 regression (exit 1, two alerts, JSON and Markdown written).

The same runner now exposes `history`, which scans a bounded newest-session suffix and emits
deterministic JSON/Markdown series plus newest-versus-previous (or fixed-baseline) regression
alerts. Tooling completion still is not performance evidence: an alert-free empty, incomplete, or
undersampled comparison is explicitly inconclusive.

- [x] Create an opt-in FAST command and bounded evidence wrapper.
- [x] Preserve the pictured-run baseline and define fixtures/fields/targets.
- [x] Implement deterministic bundle scoring with failure/timeout retention and p50/p95 output.
- [x] Implement fixed-SHA stock/FAST/manual orchestration, cold/warm cache isolation, resource/cost
  admission, and safe cleanup that never deletes shared targets.
- [x] Implement the bounded historical dashboard and configurable p50/p95, non-success, timeout,
  and validated-rate regression alerts with nonzero CI exit behavior.
- [x] Smoke the no-execution matrix planner and deterministic history regression exit path.
- [ ] Run five cold and five warm repetitions for each selected fixture and comparison lane.
- [ ] Import real manual Codex/Claude samples; do not substitute placeholders for absent runs.
- [ ] Publish raw bundle links and p50/p95 results.
- [ ] Measure escaped regressions and full-CI baseline before promoting FAST.

## How to collect benchmark evidence

### Quick start

```bash
# 1. Build the current CLI binary
cargo build -p roko-cli --bin roko --locked

# 2. Preview the benchmark matrix (dry-run, safe by default)
./scripts/run_benchmark_evidence.sh

# 3. Execute with real measurements (requires explicit opt-in)
BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh

# 4. Review the p50/p95 scorecards
./dev.sh benchmark history

# 5. Inspect individual session bundles in .roko/benchmarks/
```

### Default matrix

| Parameter | Default | Override |
|---|---|---|
| Repetitions | 5 cold + 5 warm per fixture | `REPETITIONS=3` |
| Lanes | current-roko, roko-fast | `LANES="current-roko"` |
| Max cost | $0.00 (no paid network) | `MAX_COST_USD=1.00 --allow-network` |
| Base SHA | Current HEAD | `BASE_SHA=abc1234` |

### Narrowing the matrix

For iterative development, run a single lane with fewer repetitions:
```bash
REPETITIONS=2 LANES="current-roko" CACHES="warm" BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh
```

### Underlying tools

| Tool | Entry point | Purpose |
|---|---|---|
| `scripts/dev_benchmark.py` | Direct | Benchmark orchestration with isolation |
| `./dev.sh benchmark` | Wrapper | History dashboard and regression alerts |
| `scripts/run_benchmark_evidence.sh` | Wrapper | Dev-audit evidence collection |

## Baseline from pictured run

| Metric | Baseline |
|---|---:|
| Startup to runner | about 44s including maintenance/warm |
| Actual dispatches | 2 |
| Reported calls | 1,109 |
| Mechanical T1 agent | 368.6s |
| T1 gate | 107.9s |
| T2 terminal timeout | about 600s |
| Completed plan tasks | 1 / 7 |
| Evidence correctness | TUI stale after terminal |

## Fixtures

Run each with a fixed base SHA:

1. One-line enum/string/config change.
2. Local type block with derives/imports.
3. Pure store/matching logic.
4. CLI parser plus human/JSON output.
5. HTTP endpoint behavior.
6. Persistence/concurrency invariant.
7. TUI/web visual change.

Compare:

- Manual Codex fast prompt.
- Manual Claude fast prompt.
- Current Roko.
- Roko FAST.

Run cold and warm five times each. Preserve every bundle.

The executable manifest and full operator contract are tracked under `benchmarks/dev-audit/`.
Start with `python3 scripts/dev_benchmark.py list` or a no-execution
`python3 scripts/dev_benchmark.py run --dry-run --base <commit>`, then admit paid/network lanes
only with the explicit network flag and worst-case cost ceiling documented there.

After each retained session, run `./dev.sh benchmark history`. It writes
`.roko/benchmarks/history.json` and `HISTORY.md`, compares the newest session to its immediate
predecessor by default, and exits 1 when a configured threshold is breached. Promotion should pin
the reviewed reference with `--baseline-session`; use `--fail-on-inconclusive` when CI must also
reject missing groups, missing latency, or fewer than the configured minimum samples.

## Scorecard fields

Identity:

- fixture, tier, model, effort
- base SHA
- changed files and LOC
- cache strategy and target identity

Latency:

- startup_ms
- capacity_wait_ms
- context_ms
- prompt_ms
- first_edit_ms
- agent_ms
- cargo_lock_wait_ms
- compile_ms
- targeted_test_ms
- smoke_ms
- bundle_ms
- total_ms

Provider:

- actual launches
- tool calls
- prompt/current-context/cumulative input/cached input/output tokens
- cost
- retries and timeouts

Correctness:

- dispatches per attempt
- changed-file scope
- compile/test/smoke result
- endpoint pass count
- event validity
- screenshot/evidence completeness
- terminal/exit consistency
- human intervention
- escaped regression discovered later

## Targets

| Metric | Target |
|---|---:|
| XS FAST p50 | at or below 300s |
| XS FAST p95 | at or below 600s |
| Startup p95 | at or below 5s |
| Dispatches per attempt | exactly 1 |
| Mechanical agent p95 | at or below 90s |
| Mechanical prompt/input | at or below 20k tokens unless justified |
| Warm focused compile | at or below 15–30s |
| Duplicate semantic verification | 0 |
| Post-task cargo clean | 0 |
| Terminal/evidence validity | 100% |
| Evidence bundle overhead | under 5% or 2s, whichever is larger |
| Escaped regressions | no increase |
| Full CI pass rate | at least baseline |

## Experiments

### A. Agent-owned versus runner-owned compilation

- Same fixture/model/base.
- Variant 1 lets the agent run Cargo.
- Variant 2 ends after patch and lets the runner run one command.
- Measure provider time, compile time, cache reuse, and timeout salvage.

Expected: runner-owned wins because compilation is no longer charged to/provider-limited by the
agent and is not duplicated.

### B. Cache strategy

- Shared stable worktree plus incremental target, serialized Cargo.
- Per-revision target plus CARGO_INCREMENTAL=0 and sccache/base normalization.

Measure cold/warm p50/p95, disk growth, lock waits, and hit rate. Adopt one by workload; do not keep
the ineffective hybrid.

### C. Task decomposition

- Current seven serial tasks.
- One cohesive task.
- Two coherent slices.

Measure provider launches, prompt/tool tokens, compile count, total wall time, regressions, and
reviewability.

### D. Verification policy

- Current canonical plus authored gates.
- Task-verify-only.
- Impact-selected semantic gate.

Run full CI afterward to measure escaped regressions, not only local speed.

### E. Model/effort

Use identical prompt/context and compare fast/balanced/frontier models at low/medium reasoning.
Select the cheapest/fastest model that stays within the regression threshold for each tier.

## Reporting

Publish p50, p95, cold/warm split, and raw bundle links. Never average away:

- timeouts
- failed terminal states
- human intervention
- cold-cache runs
- missing evidence

A timeout is its full deadline, not a discarded outlier.

## How to collect benchmark evidence

```bash
# 1. Build the current CLI binary
cargo build -p roko-cli --bin roko --locked

# 2. Preview the benchmark matrix (dry-run, safe by default)
./scripts/run_benchmark_evidence.sh

# 3. Execute with real measurements (requires explicit opt-in)
BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh

# 4. Review the p50/p95 scorecards
./dev.sh benchmark history
```

| Parameter | Default | Override |
|---|---|---|
| Repetitions | 5 cold + 5 warm per fixture | `REPETITIONS=3` |
| Lanes | current-roko, roko-fast | `LANES="current-roko"` |
| Max cost | $0.00 (no paid network) | `MAX_COST_USD=1.00` |
| Base SHA | Current HEAD | `BASE_SHA=abc1234` |

For a narrowed matrix: `REPETITIONS=2 LANES="current-roko" CACHES="warm" BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh`

## How to collect benchmark evidence

### Quick start

```bash
# 1. Build the current CLI binary
cargo build -p roko-cli --bin roko --locked

# 2. Preview the benchmark matrix (dry-run, safe by default)
./scripts/run_benchmark_evidence.sh

# 3. Execute with real measurements (requires explicit opt-in)
BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh

# 4. Review the p50/p95 scorecards
./dev.sh benchmark history

# 5. Inspect individual session bundles in .roko/benchmarks/
```

### Default matrix

| Parameter | Default | Override |
|---|---|---|
| Repetitions | 5 cold + 5 warm per fixture | `REPETITIONS=3` |
| Lanes | current-roko, roko-fast | `LANES="current-roko"` |
| Max cost | $0.00 (no paid network) | `MAX_COST_USD=1.00` |
| Base SHA | Current HEAD | `BASE_SHA=abc1234` |

### Narrowing the matrix

For iterative development, run a single lane with fewer repetitions:
```bash
REPETITIONS=2 LANES="current-roko" CACHES="warm" BENCHMARK_EXECUTE=1 ./scripts/run_benchmark_evidence.sh
```

### Underlying tools

| Tool | Entry point | Purpose |
|---|---|---|
| `scripts/dev_benchmark.py` | Direct | Benchmark orchestration with isolation |
| `./dev.sh benchmark` | Wrapper | History dashboard and regression alerts |
| `scripts/run_benchmark_evidence.sh` | Wrapper | Dev-audit evidence collection |
