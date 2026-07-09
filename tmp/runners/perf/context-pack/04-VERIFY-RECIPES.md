# Perf Runner — Verify Recipes

Standard verification commands referenced by every prompt's "Verify"
section. Per the global rules in `00-RULES.md`, these are run
**post-merge** (or by the runner's wave gate), **not during the
batch**. They are documented here so prompts can refer to a single
source of truth.

---

## Quick verify (for batch authors and reviewers)

After merging a batch, the runner's wave gate runs roughly:

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --release --all-targets -- -D warnings
```

The runner counts the wave as red if either fails. After the **whole
runner** completes, the test gate runs:

```bash
cargo test --workspace --release
```

If you want to validate locally before merging (optional, slow):

```bash
# Per-crate fast loop (≈15-30 s):
cargo test -p roko-runtime --lib --release
cargo clippy -p roko-runtime --release -- -D warnings

# Full workspace (≈3-8 min):
cargo test --workspace --release
cargo clippy --workspace --release --all-targets -- -D warnings
```

---

## Macro-benchmark recipes

Run after the relevant batch lands. Compare to baselines in
`tmp/solutions/perf/BENCHMARK-RESULTS.md` §3.

### Express path baseline

```bash
RUST_LOG=roko=info /usr/bin/time -l ./target/release/roko run \
  --model gpt-4.1-nano \
  --workflow-template express \
  --gates none \
  "Reply with only the word 'hello'"
```

Repeat 3 times; report median wall-time.

Expected after each phase (cumulative):

| After phase | Wall-time | Δ from baseline |
|---|---|---|
| baseline | 590 ms | — |
| Phase 0 (PERF_01..PERF_05) | 455 ms | -135 ms |
| Phase 1 (+ PERF_06) | 405 ms | -185 ms |
| Phase 2 (+ PERF_07..PERF_11) | 345 ms | -245 ms |

### Standard workflow baseline

```bash
RUST_LOG=roko=info /usr/bin/time -l ./target/release/roko run \
  --model kimi-k2-6 \
  --workflow-template standard \
  --gates express \
  "Reply with only the word 'hello'"
```

Expected after Phase 3: ~3990 ms (was ~4700-6200 ms).

### No-op repeat (validates PERF_13 source-hash skip)

```bash
./target/release/roko run --gates compile,test "noop" > /dev/null
sleep 1
/usr/bin/time -l ./target/release/roko run --gates compile,test "noop"
```

Expected (after PERF_13): second run ≥1 s faster than first.

### Plan execution (validates PERF_07 + PERF_17)

```bash
/usr/bin/time -l ./target/release/roko plan run plans/test-3-tasks/
```

Expected (after PERF_07 + PERF_17): ≥30 % wall-time reduction vs
baseline on a 10-task plan with same provider/model.

---

## Anti-pattern grep recipes

These run automatically by the runner's pre-commit hook. You can run
them locally to catch issues before pushing.

### AP-CACHE-1 (unbounded HashMap) — manual review

No automated grep. Look for `HashMap<` fields named `*_cache` or
`*_memo` and confirm an `LruCache` or hard cap exists nearby.

### AP-ASYNC-1 (sync IO in async fn) — heuristic

```bash
rg -n 'async fn' crates/ --type rust -A 30 \
  | rg -n 'std::fs::read|std::fs::write|std::fs::read_to_string|std::fs::create_dir'
```

False positives are common (the `std::fs` call may be inside a
`spawn_blocking`). Manual review of hits.

### AP-DISPATCH-1 (rogue reqwest::Client) — automated

```bash
rg -n 'reqwest::Client::(new|builder)' crates/ --type rust \
  | rg -v 'shared_http_client|SHARED_HTTP_CLIENT|tests|test\.rs'
```

Expected: empty (post-merge of any recent main).

### AP-PERSIST-1 (per-event flush) — automated

```bash
rg -n 'writeln!.*\n.*\.flush\(\)' crates/roko-runtime/src/jsonl_logger.rs
```

Expected (post PERF_04): empty.

### AP-PERSIST-2 (sequential put without batch) — automated

```bash
rg -nU 'substrate\.put\([^_].*?\.await.*?substrate\.put\([^_]' crates/ --type rust --multiline
```

Expected (post PERF_05): empty (or annotated with `// SAFETY-ORDER:`).

### AP-GATE-3 (unsafe parallel pair) — manual review

Inspect `parallel_safe_pair` body. Should match exactly the four pairs
allowed in `tmp/solutions/perf/implementation/12-parallel-gate-rungs.md`
§Step 1.

---

## Tracing-based perf assertions

For each cache, the prompt adds a `tracing::info!(target = "roko_perf",
...)` line. After the batch lands, you can sanity-check the cache is
hot by:

```bash
RUST_LOG=roko_perf=info ./target/release/roko run --gates none "hi" 2>&1 \
  | rg roko_perf
```

| Trace key | Expected count per `roko run` |
|---|---|
| `loading config from` | 1 (after PERF_01) |
| `learning_runtime_opened` | 1 (after PERF_02) |
| `PromptAssemblyService instantiated` | 1 (after PERF_06) |
| `routing cache hit:` | ≥1 on second dispatch (after PERF_07) |
| `warm slot hit:` | ≥1 on second dispatch (after PERF_10) |

---

## Test-only resets (DO NOT call from production)

Some caches expose `clear_for_test()` / `invalidate_*_cache()`
helpers gated on `#[cfg(test)]`. Use them in tests to ensure
isolation:

```rust
AgentContract::invalidate_contract_cache();        // PERF_03
PromptAssemblyService::cache_len_for_test();      // PERF_06 (test-only accessor)
```

Calling these from non-test code is an immediate red-flag in review.
