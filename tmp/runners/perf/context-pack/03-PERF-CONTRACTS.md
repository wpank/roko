# Perf Runner — Performance Contracts

These are the measurable invariants this runner promises after each
batch lands. Every prompt's `Acceptance Criteria` cross-references the
relevant contract.

---

## C-1: Single config load per CLI invocation (PERF_01)

**Contract.** A single `roko run` invocation parses `roko.toml` exactly
once.
**Measurement.**
```bash
RUST_LOG=roko_cli=trace ./target/release/roko run --gates none "hi" 2>&1 \
  | rg -c '"loading config from"'
```
Expected: `1`.
**Impact.** -30 ms per run.

---

## C-2: Single LearningRuntime open per CLI invocation (PERF_02)

**Contract.** `LearningRuntime::open_under{,_with_models}` is invoked
exactly once per `roko run`.
**Measurement.**
```bash
RUST_LOG=roko_perf=info ./target/release/roko run --gates none "hi" 2>&1 \
  | rg -c 'learning_runtime_opened'
```
Expected: `1`.
**Impact.** -70 to -100 ms per run.

---

## C-3: Contract cache is process-wide (PERF_03)

**Contract.** Each `AgentContract::load_for_role(role)` reads disk at
most once per process per role.
**Measurement.** Regression test
`load_for_role_reads_disk_only_once_per_role` in
`crates/roko-agent/src/safety/contract.rs`.
**Impact.** -30 ms per multi-tool agent turn.

---

## C-4: JSONL log buffered, not per-event flushed (PERF_04)

**Contract.** A run that emits 30 events performs ≤2 disk syncs on the
runtime-events log (1 from buffer fill + 1 from explicit flush at run
end).
**Measurement.** strace/dtruss count of `fsync`/`fdatasync` syscalls
on `runtime-events.jsonl` ≤ 2 per typical run.
**Impact.** -35 ms per run; -60 ms for chatty workflows.

---

## C-5: Substrate writes batched in hot paths (PERF_05)

**Contract.** No production code path issues ≥3 sequential
`substrate.put` calls without batching.
**Measurement.**
```bash
rg -n 'substrate\.put\(' crates/ --type rust \
  | rg -v 'put_batch|tests|test\.rs' \
  | sort | uniq -c | awk '$1 >= 3'
```
Expected: empty (each line should appear ≤2 times in the same
function).
**Impact.** -60 ms per multi-signal run.

---

## C-6: Convention detection is cached (PERF_06)

**Contract.** Two `EffectDriver::spawn_agent` calls in the same
workdir+run perform exactly **one** `std::fs::read_dir` walk of
`src/`.
**Measurement.** Test
`cached_conventions_avoid_disk_on_second_call` in
`crates/roko-compose/src/prompt_assembly_service.rs`.
**Impact.** -50 to -150 ms per multi-dispatch run.

---

## C-7: Routing decisions cached for stable inputs (PERF_07)

**Contract.** Two dispatches with the same task profile within a 5 min
window share one routing decision; the efficiency-signals file is read
at most once per 10 s.
**Measurement.** Tests `signals_are_cached_within_ttl` and
`decision_cache_hits_for_same_key`.
**Impact.** -100 to -200 ms per dispatch (additive across plans).

---

## C-8: Dispatch-time enrichment is parallel (PERF_08)

**Contract.** Per-dispatch enrichment IO completes in
`max(steps)` ms, not `sum(steps)` ms, when steps are independent.
**Measurement.** Compare wall-time of `enrich_task_context_with_search`
before/after via `tracing::info!` spans.
**Impact.** -100 to -300 ms per plan task.

**Anti-contract.** `EnrichmentPipeline::run_steps` (the 13-step plan
enrichment) **stays sequential**. Enforced by the doc-comment block
added in PERF_08.

---

## C-9: Warm pool serves second dispatch in <5 ms (PERF_09 + PERF_10)

**Contract.** `WarmDispatchPool::acquire(provider, model)` returns in
<5 µs on a warm hit; a workflow's second dispatch is a warm hit when
the first dispatch used the same `(provider, model)` pair.
**Measurement.**
```rust
let metrics = pool.metrics().await;
assert!(metrics.warm_hits >= 1);
assert!(metrics.avg_acquire_us < 5_000.0);
```
**Impact.** -20 to -50 ms per warm hit.

---

## C-10: Serve pre-warms on startup (PERF_11)

**Contract.** `roko serve` startup pre-warms one slot per
`(pre_warm_provider, pre_warm_model)` pair declared in `roko.toml`.
**Measurement.** `curl http://localhost:8080/v1/perf/warm-pool` returns
`{ "warm_hits": 0, "cold_misses": N, ... }` immediately after startup
where N matches the number of pre-warm targets.
**Impact.** -20 to -50 ms on the first user-driven dispatch.

---

## C-11: Express gate skips compile/clippy/test for non-code tasks (PERF_12)

**Contract.** `roko run --gates express` runs only the gates listed in
`EXPRESS_GATE_NAMES`. `roko run --gates auto` resolves to `Express`
when the diff contains only `.md|.toml|.yaml|.json|.txt` files.
**Measurement.** Test `express_mode_skips_compile_and_test` and
`detect_gate_mode_md_only_returns_express`.
**Impact.** -800 to -2000 ms per non-code task.

---

## C-12: Source-hash skip elides re-runs (PERF_13)

**Contract.** Two `roko run --gates compile,test "noop"` invocations
in a row, with no source change between them, run cargo only on the
first; the second emits skipped verdicts with prefix `source-hash:`.
**Measurement.** Test
`second_run_with_unchanged_source_is_skipped` and shell repeat-test in
`BENCHMARK-RESULTS.md` §11.1.
**Impact.** -500 to -1500 ms on no-op repeat runs.

---

## C-13: Compile + fmt run in parallel (PERF_14)

**Contract.** A `--gates compile,fmt` run completes in
`max(compile_ms, fmt_ms)` + scheduling overhead, not
`compile_ms + fmt_ms`.
**Measurement.** Test `compile_and_fmt_run_in_parallel`.
**Impact.** -150 ms per standard run.

---

## C-14: Single git diff per gate phase (PERF_15)

**Contract.** A workflow run with the LLM judge gate enabled spawns
`git diff` exactly **3 times** per gate-phase iteration: one each for
`HEAD` (full), `--stat HEAD`, `--name-only HEAD`. Subsequent consumers
read from the in-memory snapshot.
**Measurement.** Test `two_consumers_share_one_snapshot`.
**Impact.** -40 to -200 ms per run.

---

## C-15: Speculation hits ≥80% on standard workflow (PERF_16)

**Contract.** Over 100 standard-workflow runs, the warm pool's
`warm_hits / total_dispatches` ratio is ≥0.8.
**Measurement.** Aggregate `pool.metrics()` from
`runtime-events.jsonl` over a benchmark batch.
**Impact.** -20 to -50 ms per standard run.

---

## C-16: Plan dispatch parallel within provider caps (PERF_17)

**Contract.** A 10-task single-provider plan completes in
≈`ceil(10 / cap) * per_task_time + overhead`, not `10 * per_task_time`.
**Measurement.** Test `dispatch_group_respects_concurrency_cap`.
**Impact.** -30 to -50 % wall-time on 10+ task plans.

---

## C-17: PGO binary is faster on hot paths (PERF_18)

**Contract.** `target-pgo/release/roko` outperforms
`target/release/roko` by ≥5 % on the `cli_overhead` criterion bench
(`config_show`, `plan_validate`).
**Measurement.** `cargo bench -p roko-cli --bench cli_overhead`.
**Impact.** 5-10 % wall-time improvement on the non-IO portions of
every command.

---

## C-18: HAL wrapper produces parseable output (PERF_19)

**Contract.** `roko run --output json "hi"` emits exactly one valid
JSON object on stdout (logs to stderr); `hal/roko_agent/main.py::run`
returns a dict with all six required keys.
**Measurement.** Test
`hal/roko_agent/tests/test_main.py::test_run_returns_required_keys`.
**Impact.** Enables external benchmark calibration.

---

## C-19: Bench K-trial consistency metrics (PERF_20)

**Contract.** `roko bench swe-mini --trials 5` produces an output JSON
with `consistency.k_pass_rate`, `distribution_consistency`,
`sequence_consistency`, `cost_cv`, `duration_cv` per task.
**Measurement.** Test `multi_trial_consistency_computes_k_pass_rate`.
**Impact.** Enables HAL-style reliability tracking.

---

## C-20: bench compare exits non-zero on regression (PERF_21)

**Contract.** `roko bench compare BASE PR --threshold 20` exits with
status 1 when any task's wall-time delta exceeds 20 %.
**Measurement.** Test `bench_compare_fails_on_regression`.
**Impact.** Enables CI-driven regression detection.
