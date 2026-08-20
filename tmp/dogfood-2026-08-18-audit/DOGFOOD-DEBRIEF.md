# Dogfood Debrief: 2026-08-18 — 30-Agent Codebase Audit

> Comprehensive static audit of all 35 workspace crates using 30 parallel exploration agents, targeting reliability, security, architecture, and UX.

## Executive Summary

**30 agents** examined the full workspace across 30 audit dimensions. They found **14 CRITICAL**, **23 HIGH**, **31 MEDIUM**, and **12 LOW** issues. The most severe findings are:

1. **Dream consolidation is 100% non-functional** — tokio nested runtime deadlock
2. **Dispatch feedback projection calls non-existent methods** — provider health tracking broken
3. **ACP bridge has 16 panic!() calls** in event forwarding hot paths
4. **HTTP serve accepts unbounded response bodies** (usize::MAX) across 85+ routes
5. **Gate compile rung silently passes** when build tools aren't installed
6. **Task parser accepts duplicate IDs** — HashSet silently deduplicates
7. **Cascade router learns from manual overrides** — corrupts routing model (known: UX34)

No code was modified. This is a research-only audit for prioritization.

---

## Findings by Severity

### CRITICAL (14)

| # | Subsystem | Issue | File(s) |
|---|-----------|-------|---------|
| C01 | Dreams | `spawn_blocking` → `block_on()` creates tokio nested runtime deadlock; consolidation completely non-functional | `crates/roko-dreams/src/runner.rs` |
| C02 | Dispatch | `record_provider_success()`/`record_provider_failure()` called but don't exist on `ProviderHealthRegistry` (should be `record_success()`/`record_failure()`) — provider health tracking broken | `crates/roko-cli/src/dispatch/dispatch_v2.rs` |
| C03 | ACP | 16 `panic!()` calls in event forwarding; mutex poisoning on PermissionReplyChannel; session busy flag uses non-atomic read-then-set | `crates/roko-acp/src/bridge_events.rs` |
| C04 | HTTP Serve | `to_bytes(body, usize::MAX)` in 85+ route handlers — unbounded response body collection allows OOM | `crates/roko-serve/src/routes/*.rs` |
| C05 | Gates | Compile gate silently PASSES with warning when build tools aren't installed — defeats verification purpose | `crates/roko-gate/src/compile.rs:118-128` |
| C06 | Task Parser | Duplicate task IDs not detected — HashSet silently deduplicates; downstream assumes uniqueness | `crates/roko-cli/src/task_parser.rs` |
| C07 | Agent | `.expect()` in `HandlerFutureLifecycle::poll` panics if future is None — hot path during tool dispatch | `crates/roko-agent/src/dispatcher/mod.rs:182` |
| C08 | Agent | MCP server crash not detected mid-stream; 30s timeout blocks if child process crashes but stdout pipe stays open | `crates/roko-agent/src/mcp/client.rs:210-248` |
| C09 | Agent | Rate limiter `self.buckets.lock().expect()` panics on poisoned mutex — single panic locks all subsequent rate-limit checks | `crates/roko-agent/src/rate_limit.rs:178` |
| C10 | Learning | Override outcomes not isolated from cascade router learning (known UX34) — manual `force_backend` corrupts model | `crates/roko-learn/src/cascade_router.rs` |
| C11 | Learning | Experiment assignment not deterministic per task — pure UCB1 with no seed/task hash; unsampled variants get `f64::MAX` | `crates/roko-learn/src/model_experiment.rs` |
| C12 | Learning | Hindsight regression uses timestamp-based causality without data-flow tracking — concurrent tasks create false causal chains | `crates/roko-learn/src/hindsight.rs` |
| C13 | Plugins | Unsigned plugins accepted without verification of already-installed files; WASM panics unhandled (no `catch_unwind`) | `crates/roko-plugin/` |
| C14 | Runtime | Fire-and-forget task via `std::mem::drop(tokio::spawn(...))` — cancellation task outlives ProcessSupervisor, holds Arc to handles map | `crates/roko-runtime/src/process.rs:926` |

### HIGH (23)

| # | Subsystem | Issue | File(s) |
|---|-----------|-------|---------|
| H01 | Event Loop | 23,149-line god object; `run()` is 3,652 lines, `dispatch_action()` is 2,703 lines; 10 interleaved subsystems | `crates/roko-cli/src/runner/event_loop.rs` |
| H02 | Config | HOME env fallback to `"."` — config discovery searches CWD as home directory | `crates/roko-core/src/config/loader.rs` |
| H03 | Config | Silent global config parse failure — malformed global config silently ignored, falls through to defaults | `crates/roko-core/src/config/loader.rs` |
| H04 | Config | Env var interpolation silently empties — `${MISSING_VAR}` becomes empty string with no warning | `crates/roko-core/src/config/loader.rs` |
| H05 | Config | File secret read failure silently ignored — `read_to_string` error swallowed, secret resolves to None | `crates/roko-core/src/config/loader.rs` |
| H06 | HTTP Serve | No request body size limits on any route — clients can send arbitrarily large payloads | `crates/roko-serve/src/routes/*.rs` |
| H07 | HTTP Serve | 167 `unwrap()` + 1382 `expect()` in route handlers — any panic = 500 with no structured error | `crates/roko-serve/src/routes/*.rs` |
| H08 | HTTP Serve | Timing attack in API key comparison — string equality instead of constant-time compare | `crates/roko-serve/src/middleware/` |
| H09 | Snapshot | Corrupt snapshot = total data loss — no backup/recovery mechanism | `crates/roko-cli/src/runner/persist.rs` |
| H10 | Snapshot | `clean_stale_staging_files()` exists but is NEVER CALLED — orphaned `.tmp` files accumulate after crashes | `crates/roko-cli/src/runner/persist.rs:341-388` |
| H11 | Knowledge | Append phase not atomic with confirmations — crash between write and confirm creates phantom entries | `crates/roko-neuro/` |
| H12 | Safety | AgentContract fallback clears `allowed_tools` silently when role unknown — fails to principle of least surprise | `crates/roko-agent/src/safety/` |
| H13 | Routing | All providers circuit-open falls back to unfiltered route — cascading failure bypasses health checks | `crates/roko-learn/src/cascade_router.rs` |
| H14 | Routing | 3 consecutive failures = permanent circuit trip — no time-based recovery; expired API keys never recover from AuthFailure | `crates/roko-learn/src/cascade_router.rs` |
| H15 | Routing | Unknown model slug silently accepted — typo in config routes to wrong model with no warning | `crates/roko-learn/src/cascade_router.rs` |
| H16 | Dispatch | `RoutingObservationSink` receives `ModelChoiceSource::Override` tag but ignores it (`let _ = model_source`) — override dampening never applied | `crates/roko-cli/src/runtime_feedback/routing.rs` |
| H17 | Async | External cancellation lifetime mismatch — CancellationToken outlives the scope it protects | `crates/roko-runtime/` |
| H18 | Types | TaskStatus enum fragmented across 4 crates with incompatible variants | Multiple crates |
| H19 | Types | 4 separate EventBus implementations across workspace | Multiple crates |
| H20 | GitHub | Comment idempotency missing — repeated runs create duplicate PR comments | `crates/roko-cli/src/github/` |
| H21 | GitHub | Token expiry not handled — 401 errors not retried with refresh | `crates/roko-cli/src/github/` |
| H22 | Relay | Message loss window during cursor handoff — ACKed but not yet committed messages lost on crash | `crates/roko-serve/src/relay/` |
| H23 | PRD | Orphaned agent sidecars on PRD validation failure — process not cleaned up | `crates/roko-cli/src/prd.rs` |

### MEDIUM (31)

| # | Subsystem | Issue |
|---|-----------|-------|
| M01 | Config | TOCTOU race in config file discovery |
| M02 | Config | Hierarchical env path traversal risk |
| M03 | Config | Timeout deserialization rejects "30s" (expects integer seconds) |
| M04 | Config | Config merge doesn't deep-merge arrays (replaces entirely) |
| M05 | Task Parser | Empty plans accepted as valid (0 tasks) |
| M06 | Task Parser | Unknown TOML fields silently ignored (no `deny_unknown_fields`) |
| M07 | Task Parser | Minimal task ID character validation (accepts control chars) |
| M08 | Compose | Prompt injection risk — user input interpolated without escaping in system prompts |
| M09 | Compose | HuggingFace tokenizer fallback returns 0 tokens — budget calculations wrong |
| M10 | Knowledge | Stale snapshots during concurrent ingest — no MVCC or snapshot isolation |
| M11 | Safety | Default wildcard permissions (`*`) when no contract specified |
| M12 | TUI | File watcher accumulates events without debounce |
| M13 | TUI | JSONL truncation on very long lines (>2000 chars) |
| M14 | CLI | `roko run` exit codes not semantic (always 0 or 1) |
| M15 | CLI | `roko job execute` returns success for unimplemented marketplace commands |
| M16 | CLI | Inconsistent `--format` flag support across subcommands |
| M17 | CLI | No shell completion for dynamic values (plan names, agent names) |
| M18 | JSONL | `tool_metrics.jsonl` unbounded growth — no rotation configured |
| M19 | Gate | Concurrent gate semaphore potential deadlock under high parallelism |
| M20 | PRD | Stale frontmatter fields persist after status transitions |
| M21 | Worktree | Race between worktree creation and git operations under concurrent plans |
| M22 | Error | Silent error suppression — `let _ = result` pattern in 40+ locations |
| M23 | Error | Inconsistent context propagation — mixed `anyhow` / `thiserror` patterns |
| M24 | Relay | Reconciliation requires manual resolve — no automatic retry |
| M25 | Plugin | Incomplete semver constraint validation for dependency ranges |
| M26 | Examples | 3 of 8 graph examples broken (schema drift) |
| M27 | Examples | Missing hello-world / quickstart examples |
| M28 | Tests | roko-graph: 2 test files for 23 modules |
| M29 | Tests | roko-serve: 8 test files for 47 route modules |
| M30 | Tests | roko-compose: 2 test files for full prompt assembly pipeline |
| M31 | GitHub | Merge conflict detection but no automated resolution strategy |

### LOW (12)

| # | Subsystem | Issue |
|---|-----------|-------|
| L01 | Dead Code | 76 `allow(dead_code)` annotations — all justified (trait impls, feature-gated) |
| L02 | Worktree | Missing `.git/worktrees` cleanup on abnormal exit |
| L03 | CLI | Help text formatting inconsistent between subcommands |
| L04 | CLI | `--verbose` flag doesn't affect all output paths |
| L05 | TUI | Scrollbar rendering artifacts on very small terminals |
| L06 | Config | Provider health display truncates long error messages |
| L07 | Snapshot | Snapshot file permissions not explicitly set (inherits umask) |
| L08 | JSONL | Advisory lock timeout message could be more actionable |
| L09 | Types | `SignalKind` has 30+ variants, some with overlapping semantics |
| L10 | Deps | No unused dependencies detected (clean) |
| L11 | Examples | Config examples use hardcoded paths instead of `$HOME` |
| L12 | Tests | Some integration tests use `#[ignore]` without documented reason |

---

## Architecture Observations

### Event Loop Decomposition (H01)

The `event_loop.rs` god object is the single largest architectural debt. 23,149 lines with 10 interleaved concerns. Recommended extraction order:

1. **Bootstrap/init** → `runner/bootstrap.rs` (~400 lines)
2. **Resume/snapshot restore** → `runner/resume.rs` (~300 lines)
3. **Terminal signal handlers** → `runner/terminal.rs` (~200 lines)
4. **State mutation helpers** → `runner/state.rs` (~800 lines)
5. **Merge workflow** → `runner/merge.rs` (261 refs, ~600 lines)
6. **Telemetry emission** → `runner/telemetry.rs` (87 calls, ~400 lines)
7. **Gate lifecycle** → `runner/gates.rs` (111 refs, ~500 lines)
8. **Action dispatch** → `runner/actions.rs` (2,703 lines)
9. **Agent spawn** → `runner/spawn.rs` (~500 lines)
10. **Main loop** → stays in `event_loop.rs` (~1,500 lines)

### Cross-Crate Type Fragmentation (H18, H19)

14 type families have incompatible definitions across crates:

| Type | Crates | Issue |
|------|--------|-------|
| TaskStatus | roko-core, roko-cli, roko-graph, roko-serve | Different variant sets |
| EventBus | roko-runtime, roko-core, roko-graph, roko-serve | 4 implementations |
| ProviderKind | roko-agent, roko-core | Enum drift |
| GateResult | roko-gate, roko-cli | Field differences |

### Test Coverage Gaps (M28-M30)

| Crate | Test Files | Modules | Coverage Ratio |
|-------|-----------|---------|---------------|
| roko-graph | 2 | 23 | 8.7% |
| roko-serve | 8 | 47 | 17.0% |
| roko-compose | 2 | ~10 | 20.0% |
| roko-gate | 4 | 7 | 57.1% |

---

## Comparison with Previous Dogfood Sessions

| Session | Type | Issues Found | Fixed |
|---------|------|-------------|-------|
| 2026-04-26 | Live run | 6 critical | 6/6 |
| 2026-08-13 | Live run | 4 blockers | 4/4 (regression fixes) |
| 2026-08-17 | Examples + retest | ~12 issues | Partial |
| **2026-08-18** | **30-agent static audit** | **80 issues** | **N/A (audit only)** |

Previous sessions found runtime issues through execution. This session found latent issues through static analysis. The two approaches are complementary — many C-level issues here (C01 dream deadlock, C02 wrong method names, C05 gate bypass) would only surface under specific runtime conditions.

---

## Recommended Priority Order

### P0 — Fix before next dogfood run (blocks self-hosting)

1. **C02**: Fix dispatch method names (`record_provider_success` → `record_success`) — provider health completely broken
2. **C01**: Fix dream consolidation deadlock (`spawn_blocking` + `block_on` → `tokio::spawn` + async) — feature 100% non-functional
3. **C05**: Gate compile rung must FAIL when tools missing, not pass with warning
4. **C06**: Task parser must reject duplicate task IDs with error

### P1 — Fix before any deployment (security/reliability)

5. **C04**: Cap response body collection (`to_bytes` limit to reasonable max, e.g. 10MB)
6. **H06**: Add request body size limits to serve routes
7. **H08**: Use constant-time comparison for API keys
8. **C03**: Replace `panic!()` calls in ACP bridge with `Result` returns
9. **C07**: Replace `.expect()` in HandlerFutureLifecycle with proper error handling
10. **C09**: Replace `.expect()` on rate limiter mutex with `.lock().ok()` fallback
11. **C13**: Add WASM `catch_unwind` wrapper for plugin execution
12. **C14**: Track spawned cancellation tasks in ProcessSupervisor

### P2 — Improve reliability

13. **H09**: Add snapshot backup before overwrite (keep N-1)
14. **H10**: Call `clean_stale_staging_files()` during startup
15. **H13**: When all providers tripped, return error instead of unfiltered fallback
16. **H14**: Add time-based circuit recovery (e.g. 5-minute half-open window)
17. **H16**: Wire override dampening in RoutingObservationSink
18. **C10/C11/C12**: Learning subsystem data quality fixes
19. **M26**: Fix 3 broken graph examples

### P3 — Improve quality

20. **H01**: Begin event_loop.rs decomposition (start with bootstrap/resume extraction)
21. **H18/H19**: Consolidate cross-crate type fragmentation
22. **M28-M30**: Increase test coverage for roko-graph, roko-serve, roko-compose
23. **M08**: Escape user input in prompt templates
24. **M14-M17**: CLI UX consistency fixes

---

## Raw Audit Dimensions

30 agents examined these areas:

1. Event loop god object decomposition
2. Unwrap/expect/panic paths
3. Config parsing robustness
4. HTTP serve error handling & security
5. CLI UX consistency
6. Agent dispatch error paths
7. Gate pipeline edge cases
8. JSONL file handling
9. Snapshot persistence & recovery
10. TUI rendering edge cases
11. Test coverage gaps
12. Dead code and unused deps
13. Async/concurrency hazards
14. Task parser robustness
15. Model routing logic
16. PRD lifecycle flows
17. Worktree management safety
18. Compose/prompt assembly
19. Knowledge/neuro store integrity
20. Safety layer completeness
21. Learning subsystem data quality
22. Error type consistency
23. Dream consolidation
24. ACP integration
25. Dispatch feedback projection
26. Examples quality
27. Plugin system robustness
28. Relay/connectivity
29. Cross-crate type coherence
30. GitHub workflow integration
