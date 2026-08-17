# Perf Runner — Anti-Patterns

Concrete, perf-runner-specific footguns. Each entry has an ID so prompt
files can reference it (`AP-CACHE-3`, etc.).

---

## Caching anti-patterns

**AP-CACHE-1.** **Unbounded `HashMap` as a "cache".** Every cache must
have a hard upper bound. Use `lru::LruCache` with a stated capacity, or
a manually capped `HashMap` with an eviction step. An unbounded cache
is a memory leak with extra steps.

**AP-CACHE-2.** **No invalidation comment.** Every cache must declare
its invalidation strategy in a doc-comment above the type:

```rust
/// Cache of foo, keyed by `(workdir, mtime)`. Invalidation:
///   - mtime change on `<file>`
///   - explicit `clear_for_test()` only (test isolation)
/// TTL: none (mtime is sufficient).
```

**AP-CACHE-3.** **`static` cache for per-workdir data inside a
multi-tenant process.** `roko serve` holds many workdirs at once. A
`static` cache leaks the first-seen workdir's data to every other
tenant. Per-instance caches go on the service struct, not in a
`LazyLock`.

**AP-CACHE-4.** **Mtime-only invalidation for content that mutates
without bumping mtime.** mtime is fine for `Cargo.toml` / `src/`
directory entries (add/remove/rename of files), but it is unreliable
for `git checkout`, `cp -p`, and editors that rewrite atomically.
Pair mtime with TTL or use content hash.

**AP-CACHE-5.** **Hashing raw `f64` as a cache key.** Two reasonable
runs differ by 1e-9 in some quality score and never share a cache hit.
Bucket into deciles or fixed bins before hashing.

**AP-CACHE-6.** **Disk-persisted cache for in-process state.**
Cross-process caching is its own problem (atomicity, ABI, schema
migrations). In-process LRU + mtime invalidation almost always wins.
The exception is the source-hash gate cache, which is intentionally
disk-persisted so it survives across `roko run` invocations.

**AP-CACHE-7.** **TTL extended "to save more reads".** Long TTLs
silently break consumers (TUI dashboards, A/B reports) that depend on
fresh data. Keep TTLs short (≤10 s for hot data).

---

## Async / concurrency anti-patterns

**AP-ASYNC-1.** **`std::fs::*` inside an `async fn`.** Blocks the
Tokio runtime; tail latency spikes during heavy concurrency. Wrap in
`tokio::task::spawn_blocking`, or migrate to `tokio::fs::*`.

**AP-ASYNC-2.** **Holding a sync mutex (`std::sync::Mutex`,
`parking_lot::Mutex`) across `.await`.** Common cause of deadlocks
and starvation. Either drop the guard before the await
(`{ let g = m.lock(); g.foo() }`) or switch to
`tokio::sync::Mutex`.

**AP-ASYNC-3.** **Unbounded `tokio::spawn` in a hot path.** Spawning
N tasks concurrently smashes provider rate limits and the OS scheduler.
Bound with `tokio::sync::Semaphore` (`acquire_owned()`) or
`futures::stream::buffer_unordered`.

**AP-ASYNC-4.** **`tokio::join!` over interdependent futures.** Joins
are concurrency, not parallelism, but they still execute branches in
arbitrary order with no happens-before. Use only when branches are
truly independent.

**AP-ASYNC-5.** **`futures::future::join_all` for fixed-arity joins.**
Use the macro-based `tokio::join!` (no allocation) when the arity is a
compile-time constant. Reserve `join_all` for runtime-dynamic lengths.

**AP-ASYNC-6.** **Spawning a future and `.await`ing it.** That defeats
the speculation; the spawn moves the future to a different task and the
await blocks anyway. Use `tokio::spawn` only when you genuinely want
fire-and-forget (e.g., warm-pool pre-warm) OR when you need a different
runtime context.

---

## Persistence anti-patterns

**AP-PERSIST-1.** **`flush()` per event in a JSONL log.** The OS
already buffers; a `BufWriter` + explicit flush at end-of-run is the
contract. Per-event flush turns a 100 µs write into a 3 ms one.

**AP-PERSIST-2.** **Writing two `put`s in sequence when a `put_batch`
would do.** Each `put` serialises, appends, and flushes. Two of them =
two of each. `put_batch` does it once. The `FileSubstrate::put_batch`
API exists for this reason.

**AP-PERSIST-3.** **Hand-rolling atomic writes.** Use
`roko_fs::atomic::atomic_write_bytes` / `atomic_write_json`. They do
the write-tmp-rename dance. Hand-rolled writes corrupt the file on a
mid-write crash.

**AP-PERSIST-4.** **Failing the gate run on a hash-cache write
error.** The cache is best-effort. If the disk is full or read-only,
emit `tracing::warn!` and proceed; do not abort the gate.

---

## Provider / dispatch anti-patterns

**AP-DISPATCH-1.** **Constructing `reqwest::Client::new()` in a hot
path.** The `SHARED_HTTP_CLIENT` static at
`crates/roko-agent/src/provider/mod.rs:88` is the only legal client
factory. New per-call clients destroy the connection pool and add
TLS handshake latency.

**AP-DISPATCH-2.** **Calling `create_agent_for_model` per dispatch
when the warm pool is available.** The pool exists to amortize
agent-construction cost; bypassing it is the regression we are fixing.

**AP-DISPATCH-3.** **Caching `Arc<dyn Agent>` keyed only by provider.**
Two requests to the same provider with different models must not
share an `Agent` (different routing/temperature/etc.). Key by
`(provider, model)`.

**AP-DISPATCH-4.** **Pre-warming in `roko run` (CLI one-shot).** The
first dispatch's TLS handshake is the same with or without
pre-warming; pre-warming costs latency without the win for one-shot
invocations. Pre-warm only in `roko serve`.

**AP-DISPATCH-5.** **Releasing a warm slot via `Drop`.** `Drop` cannot
be async. Release explicitly with `pool.release(idx)` after the call.

---

## Gate-pipeline anti-patterns

**AP-GATE-1.** **Skipping rung 0 (compile) in any mode that touches
code.** `--gates none` is for non-code tasks only; `--gates auto` must
escalate to `Full` whenever code files are modified. Skipping compile
silently ships broken code.

**AP-GATE-2.** **Bypassing the adaptive-threshold mechanism with the
new mode/source-hash mechanisms.** All three skip mechanisms compose
orthogonally. Adaptive-threshold can skip a gate that mode allowed.
Source-hash can skip a gate that adaptive-threshold ran. Each gets
its own `skip_reason` so audits can tell them apart.

**AP-GATE-3.** **Parallelising compile + clippy or compile + test.**
Both invoke `cargo`; they contend for the workspace lock. Net win:
zero. Net cost: confusing logs. Only the `(compile, fmt)` and
`(clippy, fmt)` pairs are safe.

**AP-GATE-4.** **Re-ordering verdicts in the output report after
parallel-group execution.** Consumers (CLI, serve, learning loop)
rely on rung-order in the verdict list. Re-sort by rung after the
parallel join.

**AP-GATE-5.** **Removing the compile-failure short-circuit.**
Running test/clippy after a compile failure is 10+ seconds of
wasted work. The dependency must be preserved across parallel
groups.

**AP-GATE-6.** **Caching failure verdicts in the source-hash cache.**
Only cache successful runs. A failed run's source state isn't a
meaningful "skip" candidate.

**AP-GATE-7.** **Including `target/`, `node_modules/`, or `.roko/`
in the gate input set.** Build artefacts change every build and
invalidate the cache pointlessly. Use the `ignore` crate to respect
`.gitignore`.

---

## Routing / learning anti-patterns

**AP-ROUTE-1.** **Cross-process routing decision cache.** Each `roko
run` is a fresh process; cross-process caching needs disk persistence,
which costs more than the routing decision itself. In-process only.

**AP-ROUTE-2.** **Cache without mtime invalidation.** A 10 s TTL alone
misses the case where a parallel `roko run` writes a new efficiency
signal inside the same window. Always pair TTL with mtime check.

**AP-ROUTE-3.** **Write-through cache that updates on every observe
call.** Couples write paths to cache state and gets racy fast. Mtime +
TTL invalidation is the only sane design.

**AP-ROUTE-4.** **Caching the full `RoutingExplanation`.** Deeply
nested vectors of model scores. Cache the chosen model + a short blob;
re-derive the full explanation on demand.

---

## Enrichment anti-patterns

**AP-ENRICH-1.** **Parallelising `EnrichmentPipeline::run_steps`.**
The 13 steps in `ALL_ORDERED` consume each other's output files. Naive
`tokio::join!` produces empty/inconsistent prompts. Sequential is
required; the doc-comment in `pipeline.rs` makes this explicit.

**AP-ENRICH-2.** **`tokio::join!` of futures with disparate timeouts.**
A 30 s research fetch joined with a 10 ms file read makes the whole
dispatch wait 30 s. Wrap long-tail enrichers in
`tokio::time::timeout(short, ...)` first.

**AP-ENRICH-3.** **`tokio::spawn` for enrichment IO.** Spawning moves
to a different task; for short, dispatch-time IO you keep the current
task and avoid the scheduler hop. `tokio::join!` is the contract.

---

## CLI / serve anti-patterns

**AP-CLI-1.** **New CLI flag without `value_enum` / docstring.** Every
new flag must have a clap doc comment and use `value_enum` for
constrained choices.

**AP-CLI-2.** **Logging to stdout in `--output json` mode.** The HAL
wrapper assumes the entire stdout is parseable JSON. Push logs to
stderr in JSON mode.

**AP-CLI-3.** **Default-on flag for an experimental feature.**
`--batch-async`, `--no-gate-cache`, `--gates none` are debug aids; do
not promote them to defaults.

**AP-CLI-4.** **Dangerous default in `roko init`.** Fresh workspaces
must default to safe values. Anything that allows skipping permission
checks defaults to `false`.

---

## Measurement / benchmark anti-patterns

**AP-BENCH-1.** **Defaulting `trials > 1`.** Multi-trial multiplies
API spend; it must be opt-in.

**AP-BENCH-2.** **`f64::NAN` in serialised output.** Most JSON parsers
choke; downstream tooling breaks. Use `Option<f64>` and emit `null`.

**AP-BENCH-3.** **Comparing across different model configs in `bench
compare`.** Add a sanity check: `baseline.config_hash !=
candidate.config_hash` warns and requires `--force`.

**AP-BENCH-4.** **PGO training with synthetic micro-benchmarks.** PGO
data must reflect *user workloads*. Use `roko config show`, `roko run
--gates none`, `roko bench demo` — not `cargo bench` loops.

**AP-BENCH-5.** **PGO build on `pull_request` events.** Each
instrumented build + training takes 5-10 min of CI time. PGO belongs
on `push: branches: [main]` or manual trigger.

---

## When you spot one in code

If your batch's edit window contains an existing instance of one of
these anti-patterns, **leave it alone unless the prompt explicitly says
to fix it.** File a follow-up note in the commit trailer:

```
followup: AP-ASYNC-2 instance at crates/X/src/Y.rs:NNN; suggest
opening PERF_FOLLOWUP_<topic>.
```
