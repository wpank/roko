# Anti-patterns specific to productionizing batches

## H — Hardening

H-1. **`expect("poisoned")` is a panic.** Replace with `unwrap_or_else(|p| { tracing::warn!("...lock poisoned, recovering"); p.into_inner() })`.

H-2. **`unwrap()` on `Mutex::lock()` is a panic.** Same fix.

H-3. **`let _ = ...` on a `Result` outside intentional cleanup is a silent swallow.** Convert to `if let Err(e) = ... { tracing::warn!(error = %e, "..."); }`.

H-4. **Holding `std::sync::Mutex` across `.await` deadlocks.** When you need a mutex inside async code, do the lock+work synchronously and drop the guard before any `await`, or use `tokio::sync::Mutex`.

H-5. **`flock` is not async-safe.** Wrap `flock` calls in `tokio::task::spawn_blocking` if they sit on the hot path. Or, since the ops are short, hold them in a sync block. Never `await` while a flock is held.

H-6. **Hardcoded model strings ARE bugs.** Even `"claude-sonnet-4-6"` shows up via `.unwrap_or_else(|| "claude-sonnet-4-6".into())` — replace with `config.agent.default_model.clone()`.

H-7. **`tower_http::timeout` only times out the future, not the inner operation.** Long operations (plan run, bench) need to honor cancellation. Don't crank the timeout up to "long enough"; use `.route_layer(...)` to exempt long routes.

H-8. **The body limit is per-route override-able.** 32 MiB at the top is fine for general routes; uploads (`/api/config`, plan files) might need higher. Don't lower the global limit to fix a specific route — exempt that route.

H-9. **`/health` is a contract.** Top-level `/health` (`crates/roko-serve/src/routes/mod.rs:193`) returns 200 unconditionally. Do NOT change it. If you want a discriminating probe, change `/api/health`.

H-10. **`Dockerfile.runtime` does NOT include the Rust toolchain.** Don't `COPY --from=builder /usr/local/cargo` into it. The point of P16 is no toolchain in image.

H-11. **Don't include `target/` in `.dockerignore` for `Dockerfile.runtime`.** The runtime image needs the cross-compiled binary inside `target/x86_64-unknown-linux-gnu/release/`.

## F — Frontier

F-1. **`MultiAgentPool` is not for fan-out alone — it's for warm reuse + fan-out.** Don't replace it with a fresh pool per task.

F-2. **HDC fingerprints are deterministic.** Same input → same fingerprint. Don't hash twice with different RNG seeds; reuse the existing `hdc_fingerprint` helpers.

F-3. **Sheaf inconsistency is a *signal*, not a gate.** F03 should *log* the score; it must not block routing.

F-4. **Collusion detection is async + advisory.** F07 must not await on the detector before assignment; it should `tokio::spawn` the check or run on a separate timer.

F-5. **Don't add a runtime dependency on `roko-chain` from `roko-cli`.** The chain crate stays out of the hot path. F07's wiring lives entirely inside `roko-chain`.

F-6. **Curriculum modes don't replace each other.** F01 adds `CurriculumMode::Adas` alongside the four existing modes — it does not modify the others.

F-7. **`pub mod` in `lib.rs` is alphabetical.** Insert new modules in alphabetical order to minimize merge conflicts.

## D — Production economics

D-1. **`f64` atomics don't exist in stable Rust.** Store dollar amounts in cents as `AtomicU64`.

D-2. **`CostsDb` is JSONL on disk.** D03 and D06 should query the existing schema, not introduce a new format.

D-3. **Semantic cache is in-memory only.** D02 must not persist to disk — it rebuilds quickly from natural traffic.

D-4. **Budget guards are pre-flight only.** D01 enforces *before* dispatch. After the LLM call, spend is recorded; over-spend triggers the guard on the *next* dispatch, not this one.

D-5. **OpenTelemetry is feature-gated.** D08 must not pull `opentelemetry-*` crates in the default build. Use `#[cfg(feature = "otel")]` on every site that touches OTEL types.

D-6. **`compliance_export` does not include raw prompts/responses.** Metadata only — model, cost, tokens, gate verdict, timestamps. PII or secrets in audit dumps are a worse problem than the missing audit dump.

D-7. **Competitive baselines are static.** D09 must not call competitor APIs. Numbers are manually maintained from public sources, with citations.

D-8. **Don't reorder the cache lookup chain.** Order is: exact match → semantic match → dispatch. Reversing gives semantic hits priority over exact ones, which is wrong (exact is free, semantic costs CPU).

## All groups

X-1. **Don't introduce a new dependency without justification.** Most things you need are already in the workspace. Check `Cargo.lock` before adding.

X-2. **Don't refactor `orchestrate.rs` beyond the named lines.** It's a 22kLOC file. Touch only the lines listed in the prompt.

X-3. **Don't migrate to `tokio::sync::Mutex` "while you're there".** Mixing sync/async mutexes is a source of deadlocks. If the existing code uses `std::sync::Mutex`, keep it; just recover from poisoning.

X-4. **Test fixtures are NOT production code.** Lines that contain `#[cfg(test)]`, `#[test]`, or are inside `mod tests {}` blocks are exempt from "no hardcoded model" rules.

X-5. **`pub use` in `lib.rs` matters for the runner.** When you add a module that other crates need to import (e.g. `bench_history` for D04 → `roko-serve` consumer), also add `pub use bench_history::{...};` if there are external users.
