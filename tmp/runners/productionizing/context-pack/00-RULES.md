# Productionizing Rules

## CRITICAL: Do NOT compile or run tests

**DO NOT run any of these commands:**
- `cargo check`, `cargo build`, `cargo test`, `cargo clippy`, `cargo run`
- `rustc`, `rustfmt`, `cargo fmt`
- Any compilation or test execution

**WHY:** Compilation is handled by a separate validation pipeline AFTER your changes are merged. Running cargo wastes significant time and resources. Just write correct code and commit it. If you need to understand types or signatures, READ the source files instead of compiling.

## Universal Anti-Patterns (carry forward from post-parity)

- A second provider resolution chain.
- A second prompt assembly path for the same mode.
- A second chat/session state owner.
- Raw provider HTTP in CLI code when an adapter exists.
- Demo data shown as live data.
- Unknown usage recorded as zero.
- Stub gate counted as pass.
- A new top-level crate for behavior that already exists in a current crate.
- Broad `orchestrate.rs` refactors mixed with behavior changes.

## Productionizing Anti-Patterns

PR-1. **Wire, don't build.** Most "missing" items in plan 11/12 already have substrate. Search the workspace before adding anything new. Examples:
  - Cost tracking → already in `roko-learn/src/{costs_db,costs_log,cost_table}.rs`
  - Response cache → already in `roko-agent/src/cache.rs`
  - Cascade router → already in `roko-learn/src/cascade_router.rs`
  - Episode logging → already in `roko-learn/src/episode_logger.rs`
  - HDC → already in `roko-primitives/src/hdc.rs`
  - Bandits → already in `roko-learn/src/bandits.rs`

PR-2. **Provider availability has a real API.** Do not write `std::env::var("ANTHROPIC_API_KEY").is_ok()` inline. Call `RokoConfig::is_provider_available(p)`, `provider_available_for_model_key(slug)`, or `available_provider_ids()` (`crates/roko-core/src/config/schema.rs:440–482`). All three already account for `[agent.env]` overrides and CLI-backed providers (Claude CLI, Cursor ACP).

PR-3. **No silent error swallowing.** Every `let _ = ...` and `.ok()` outside intentional cleanup needs a `tracing::warn!` or `tracing::error!` line. The plan 10 audit (`tmp/productionizing/06-AUDIT-FINDINGS.md` H3) lists 25+ remaining sites; only fix the ones the prompt names — don't try to hit them all.

PR-4. **No `unwrap()` or `expect()` in any new code.** Use `?` + `anyhow::Context`. For mutex poisoning specifically, prefer `.unwrap_or_else(|p| p.into_inner())` with a `tracing::warn!`.

PR-5. **No new top-level crates.** All work fits in existing crates. If you think you need a new crate, you're wrong — search harder.

PR-6. **Hardcoded model strings = bug.** Replace `"claude-sonnet-4-6"`, `"claude-haiku-4-5"`, etc. with `config.agent.default_model.clone()` or with reads from the `[models.*]` table. Test fixtures and schema-version constants are exempt.

PR-7. **Inter-process state writes need flock.** `tokio::sync::Mutex` is intra-process only. Anything that opens `.roko/episodes.jsonl`, `.roko/efficiency.jsonl`, `.roko/learn/cascade-router.json`, `.roko/state/executor.json` from multiple processes needs `flock(2)` (or equivalent on non-Unix). Use `libc::flock` directly — no new crates.

PR-8. **Auto-rotate respects existing GC logic.** The codebase already has `roko_fs::gc::GcEngine::should_auto_gc()` and a full GC pipeline. Auto-rotation triggers the existing engine; it does not delete files itself.

PR-9. **Health endpoints are stable contracts.** `/health` (top-level) is a minimal liveness probe and **always** returns 200. `/api/health` carries the richer status and may return 503 when degraded. Do not change which is which — Railway and existing dashboards depend on them.

PR-10. **Auth-disabled-on-public-bind is already an error.** `validate_bind_safety` (`crates/roko-serve/src/lib.rs:641–660`) rejects this case unless `acknowledge_public_risk = true`. Do not weaken that check.

PR-11. **Frontier code is observation-only by default.** ADAS, novelty search, sheaf inconsistency, collusion detection are advisory signals. They log and persist; they do not block dispatch. Wire them as opt-in (`learning.{name}_enabled = true` in roko.toml).

PR-12. **Persist learning state on the existing patterns.** Use the JSON pattern from `cascade_router::save/load` for opaque state, the JSONL pattern from `costs_log` for streaming events, and the binary `bincode`/`postcard` pattern only when explicitly told (HDC archives). Do not invent a new format per task.

## Coding Style

- `tracing::*` macros for logs, never `println!`/`eprintln!` outside CLI direct-output.
- Structured fields where reasonable: `tracing::info!(plan_id = %id, cost = ?cost, "plan completed")`.
- `anyhow::Result` for application code; `thiserror`-derived errors for library APIs (see `RokoError`).
- `clippy::all`, `clippy::pedantic`, `clippy::nursery` should still pass. Do not add `#[allow(...)]` to suppress findings — fix them.

## Wave Discipline

- Each batch touches only files in its `scope = [...]`. Do not modify files outside scope.
- A `deps = ["X"]` batch is **only** scheduled after X commits land. Never read-ahead.
- If a batch's prompt mentions a file you can't find, stop and report (do not invent the file).
