# Perf Runner — Global Rules

## CRITICAL: Do NOT compile or run tests during a batch

**DO NOT run any of these commands while implementing a batch:**

- `cargo check`, `cargo build`, `cargo test`, `cargo clippy`, `cargo run`
- `rustc`, `rustfmt`, `cargo fmt`, `cargo bench`
- Any compilation, test execution, or benchmark run.

**WHY:** Compilation is handled by the runner's wave gate (`cargo
check + clippy` after each group) and by a separate validation pipeline
that runs after batches merge. Spinning up cargo per batch wastes
several minutes of wall-time and CI budget. The runner already takes
care of it.

**For perf work specifically:** macro-benchmarks live in
`tmp/solutions/perf/BENCHMARK-RESULTS.md` §11 and run as a separate
post-merge step. Do not try to measure during the batch — you do not
have a stable baseline.

If you need to understand types or signatures, **READ** the source
files instead of compiling.

---

## Universal anti-patterns

These apply to every batch in this runner.

- A second copy of code that already exists somewhere in the workspace.
  Reuse the existing module.
- A new top-level crate for behaviour that fits inside an existing
  crate.
- A broad `orchestrate.rs` refactor mixed with a behaviour change.
  Refactor in one PR; behaviour in another.
- Stub gate counted as pass.
- Unknown usage recorded as zero.
- Demo data shown as live data.
- Process success treated as artifact success.
- Synchronous `std::fs` IO inside an async function (blocks the Tokio
  runtime). Use `tokio::fs` or wrap in `tokio::task::spawn_blocking`.
- Holding a `std::sync::Mutex` (or `parking_lot::Mutex`) guard across
  `.await`. Use `tokio::sync::Mutex` or restructure to drop the guard
  first.
- Adding a `static` cache for per-workdir data inside `roko serve`
  (cross-tenant pollution).
- Unbounded caches (`HashMap<K, V>` with no eviction policy).
- Caches without an invalidation strategy comment.
- New `unwrap()` / `expect()` in non-test code. Use `?` or `let-else`.
- New `reqwest::Client::new()` outside the existing `SHARED_HTTP_CLIENT`
  static at `crates/roko-agent/src/provider/mod.rs:93-110`.

---

## Perf-runner-specific rules

### R-1. Cache hygiene (covered in detail in `02-ANTI-PATTERNS.md`)

Every cache must declare its invalidation strategy in a doc comment
(TTL, mtime, content-hash, scope-bound). Every cache must have a hard
upper bound (`LruCache` capacity or explicit cap).

### R-2. Tracing hooks for measurability

After adding any optimization, instrument it with `#[tracing::instrument(skip_all)]`
or an `tracing::info!(target = "roko_perf", ...)` line so future
benchmarks can detect regressions and attribute deltas to the right
component.

### R-3. Measurement boundaries

Add timing facts to `tmp/solutions/perf/BENCHMARK-RESULTS.md` only by
running the macro-benchmark in §11.1, NOT by guessing. If a batch
unblocks a measurement, do the measurement after merge — not during
the batch.

### R-4. Backward compatibility for events / configs / public types

- Adding a new field to `RuntimeEvent::*` variants: must default to a
  serde-friendly empty value so old `runtime-events.jsonl` files still
  parse.
- Adding a new field to a config schema: must have `#[serde(default)]`
  with a sensible default; existing `roko.toml` files must continue
  loading.
- Adding a new public method on a trait: provide a default impl so
  external implementors do not break.

### R-5. Stay inside the prompt's write scope

The `scope` field in `batches.toml` is the **complete** list of files
your batch is allowed to modify. Do not edit anything else, even if you
spot a related issue. File a follow-up note in the commit message
trailer:

```
followup: noticed that crates/X/src/Y.rs has a similar bug; suggest
opening PERF_NN_FOLLOWUP for it.
```

### R-6. Delete code that the change makes redundant

If the optimization removes the need for an old code path (e.g., the
config-cache plan removes 4 redundant `load_config` calls), delete the
old code in the same PR. Do not leave dead branches "for safety".

### R-7. Tracker trailer in commit message

Every commit that closes a batch must end with a trailer line:

```
tracker: PERF_NN done <commit-sha>
```

The runner's post-merge sync script (or manual edit) flips the
checkbox in `ISSUE-TRACKER.md` based on this trailer.

---

## Communication / commit format

- Subject: `perf(<crate>): <action> (PERF_NN)`. Examples:
  - `perf(cli): cache config bundle for the entire run (PERF_01)`
  - `perf(runtime): WarmDispatchPool module (PERF_09)`
  - `perf(gate): express + auto gate modes (PERF_12)`
- Body (required): one paragraph explaining what changed and the
  expected wall-time impact (cite the plan §).
- Trailer (required): `tracker: PERF_NN done <sha>`.

---

## When you finish

1. Re-read your diff against the prompt's `Write Scope` and `Acceptance
   Criteria`. Anything outside scope = revert.
2. Confirm tracking-tracer hooks are in place (`R-2`).
3. Add the commit-message trailer (`R-7`).
4. The runner will pick up the next ready batch.
