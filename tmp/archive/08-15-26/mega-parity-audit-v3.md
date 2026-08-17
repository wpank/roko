# Mega-Parity Audit v3 — Full Quality Report

**Date:** 2026-04-29
**Branch:** wp-arch2
**Scope:** All 130 committed tasks across R2–R7 + D8
**Method:** 7 parallel agents reading every source file in scope

---

## Executive Summary

130 of 147 tasks committed (88%). Of those 130:

| Rating | Count | Meaning |
|--------|-------|---------|
| SOLID | 90 | Works as described, wired, tested |
| PARTIAL | 37 | Implemented but with meaningful gaps |
| HOLLOW | 3 | Claimed done but fundamentally broken or non-functional |

**3 runtime crashes found. 1 security hole found. 8 structural anti-patterns identified.**

---

## HOLLOW Tasks (3) — Claimed Done, Actually Broken

### 1. R7_E03: `roko config mcp` panics at runtime
**File:** `crates/roko-cli/src/main.rs` + `crates/roko-cli/src/commands/config_cmd.rs`
**Issue:** `ConfigCmd::Mcp` falls through to `dispatch_config()` which hits `unreachable!("mcp dispatched in dispatch_subcommand")`. The intercept in `main.rs` was never written. Running `roko config mcp list` crashes with a Rust panic.
**Fix:** Add `ConfigCmd::Mcp { cmd }` match arm in main.rs, implement dispatch handler.

### 2. R3_E01: Stub API provider turn — not a stub
**File:** `crates/roko-cli/src/chat_session.rs`
**Issue:** The "stub" is just a guard clause returning `SessionError::ApiProviderNotImplemented`. There is no `send_turn_api` function, no scaffolded API call sequence. The `api_history: Vec<ChatMessage>` and `http_client: reqwest::Client` fields exist but are completely unused. A future implementer would need to invent the API dispatch path from scratch.

### 3. R4_C05: Feed validation errors into plan regenerate — not implemented
**File:** `crates/roko-cli/src/commands/plan.rs`
**Issue:** `PlanCmd::Regenerate` runs the agent then validates after the fact. If validation fails, it restores the old file and bails. It does NOT collect PLAN_030/031/032/033 diagnostics, does NOT inject them as context into the regeneration prompt. The regeneration agent is blind to what went wrong.

---

## Security Findings (4)

### CRITICAL: Unauthenticated share creation (missing R6_C01)
**File:** `crates/roko-serve/src/routes/shared_runs.rs`
`POST /api/runs/{id}/share` is mounted in `shared_runs::routes()` at the OUTER router, OUTSIDE the `nest("/api", api)` block where `require_api_key` is applied. Even with `serve.auth.enabled = true`, any unauthenticated caller can create a share for any run ID.

### HIGH: Auth is opt-in, stock deploys wide open (missing R6_A01)
Default bind is `127.0.0.1` (correct), but `roko init --cloud` sets `0.0.0.0`, `PORT` env overrides to `0.0.0.0`, and no auth is auto-provisioned. Users who deploy with `roko deploy railway` get a public bind and must manually configure `serve.auth`.

### MEDIUM: `acknowledge_public_risk` bypasses terminal auth silently
**File:** `crates/roko-serve/src/routes/mod.rs`
`terminal_requires_auth` does NOT check `api_auth.enabled`. Setting `acknowledge_public_risk = true` gives an unauthenticated terminal on a public bind.

### LOW: CLI Gist path has zero scrubbing (R6_C02 partial)
**File:** `crates/roko-cli/src/share.rs`
Server-side share scrubbing works (shared_runs.rs). But the `roko run --share` Gist upload path sends raw `report.output_text` to GitHub verbatim. If an agent echoes an API key, it appears in the Gist.

---

## Structural Anti-Patterns (8)

### 1. Two parallel model selection paths
`model_selection.rs` (6-step precedence) is used by `roko run`, `prd`, `plan generate`, `config`. But `orchestrate.rs` per-task dispatch has its own inline 8-step pipeline via cascade router + role lookup. R2_B05 only wires `model_selection.rs` for override validation, not the main routing.

### 2. Streaming events silently drained in Session mode
`chat_inline.rs` `dispatch_prompt` creates a streaming event channel then drains it: `while let Some(_event) = event_rx.recv().await {}`. The TUI stays in `Phase::Thinking` (spinner) throughout. Three R3 tasks (C02, B03, partially D01) are wired to a pipeline that is then blocked. Text appears only after the full turn completes.

### 3. Context pack not wired into plan generation
`build_repo_context` is called from `prd draft new` but NOT from `plan generate`, `plan regenerate`, or `prd plan`. R4_A06 is half-implemented. Plans are generated without repository grounding.

### 4. Config schema split persists
`roko init --profile rust` emits `[[gate]]` TOML arrays. `RokoConfig::from_toml()` reads `[gates]` struct with booleans. Init output is incompatible with runtime config parsing.

### 5. Duplicate helper functions
`resolve_effective_model_key()` is duplicated between `commands/prd.rs` and `commands/plan.rs`. `ArtifactValidationReport` is two completely different types in `commands/prd.rs` (struct) and `runtime_feedback.rs` (type alias for `serde_json::Value`).

### 6. Dual demo data with no schema sync
Rust `demo_seed.rs` generates real typed JSONL artifacts. TypeScript `demo-data.ts` has hardcoded parallel objects. No shared schema — they will drift silently.

### 7. `gate_for_name("shell")` returns a stub
`gate_service.rs` line 77: `ShellGate::new("true", vec![])`. Only `run_gates()` correctly pulls from config. Anyone calling `gate_for_name("shell")` directly gets a gate that always passes. TODO comment acknowledges this.

### 8. Pipeline template hardcoded as "standard"
`cmd_run` in `util.rs` always passes `"standard"` to the workflow engine. The band config from `roko.toml [pipeline]` is never reached for `roko run`. TODO(W03) comment.

---

## Per-Runner Detailed Findings

### R2: Execution Contract (23/30 committed)

| Task | Rating | Key Finding |
|------|--------|-------------|
| R2_A02 init schema v2 | SOLID | `#![allow(dead_code)]` module-wide |
| R2_A03 migrate --yes | SOLID | — |
| R2_A05 real gate programs | SOLID | — |
| R2_A06 validate warnings | SOLID | — |
| R2_B02 EffectiveModelSelection | SOLID | CascadeRouter invoked with empty features (documented) |
| R2_B03 wire into roko run | SOLID | — |
| R2_B04 wire into prd/plan | SOLID | Helper duplicated across two files |
| R2_B05 wire into plan run | PARTIAL | Only override validation uses module; per-task routing is ad-hoc |
| R2_B06 wire into config | SOLID | — |
| R2_B07 print/persist selection | PARTIAL | No persistence artifact; selection is ephemeral |
| R2_C02 shell gate wiring | PARTIAL | `gate_for_name("shell")` returns `true` stub; only `run_gates()` is correct |
| R2_C03 gate config passthrough | SOLID | — |
| R2_C04 skipped verdicts | SOLID | — |
| R2_C05 skipped not pass | SOLID | — |
| R2_C06 gate regression test | SOLID | — |
| R2_D02 explain nonzero | PARTIAL | Global atomic flag; exit depends on call ordering |
| R2_D03 validate before run | SOLID | — |
| R2_D04 --fresh flag | SOLID | — |
| R2_D06 status agreement | SOLID | — |
| R2_E02 learn path align | SOLID | — |
| R2_E03 learn path test | SOLID | — |
| R2_F01 no raw JSON dump | SOLID | — |
| R2_F02 JSON 404 | SOLID | — |
| R2_G01 pipeline template | PARTIAL | Template hardcoded as "standard" via TODO(W03) |
| R2_G02 model to EffectDriver | SOLID | — |

### R3: Agent Session Parity (17/22 committed)

| Task | Rating | Key Finding |
|------|--------|-------------|
| R3_A01 ChatAgentSession struct | SOLID | `settings_json` never set in `new()` |
| R3_A02 system prompt | SOLID | — |
| R3_A03 tool policy | SOLID | Contract-found happy path untested |
| R3_A04 MCP config | SOLID | — |
| R3_A05 slash commands | SOLID | — |
| R3_B01 send_turn | SOLID | — |
| R3_B02 session_id capture | PARTIAL | Non-streaming path always returns None |
| R3_B03 tool output capture | PARTIAL | Events drained silently on Session dispatch path |
| R3_B04 cancellation/timeout | SOLID | — |
| R3_B05 turn unit tests | SOLID | — |
| R3_C01 stream-json parsing | SOLID | — |
| R3_C02 forward text deltas | PARTIAL | `render_stream_event` is dead on live Session path |
| R3_C03 capture metadata | SOLID | Redundant double-set of session_id |
| R3_C04 streaming proof | PARTIAL | Tests in chat_session.rs module, not tests/ dir |
| R3_D01 route through session | PARTIAL | Wired but streaming events silently drained; spinner-only |
| R3_E01 API provider stub | HOLLOW | Guard clause only; no scaffold for future implementer |

### R4: Plan Grounding (18/24 committed)

| Task | Rating | Key Finding |
|------|--------|-------------|
| R4_A01 RepoContextPack | SOLID | — |
| R4_A02 workspace/project kind | SOLID | — |
| R4_A03 key files/symbols | SOLID | — |
| R4_A04 related PRDs/plans | SOLID | — |
| R4_A05 context-root mismatch | SOLID | — |
| R4_A06 build context pack | PARTIAL | Only wired into `prd draft new`, NOT `plan generate` |
| R4_B01 inject into prd draft | SOLID | — |
| R4_B02 require grounding section | PARTIAL | Advisory warning only, doesn't reject or fail |
| R4_B03 validate PRD references | PARTIAL | Only checks duplicate crates, not referenced files |
| R4_B04 persist sidecars | SOLID | Context sidecar drops key_files/symbols/related_prds |
| R4_C01 require role field | SOLID | — |
| R4_C02 normalize model aliases | PARTIAL | Warn-only at validation; executor doesn't normalize |
| R4_C03 validate file refs | SOLID | — |
| R4_C04 reject duplicates | SOLID | — |
| R4_C05 feed errors into regen | HOLLOW | `plan regenerate` does not inject diagnostics |
| R4_C06 separate artifact validation | SOLID | — |
| R4_D01 withhold knowledge seeds | SOLID | Only fires via orchestrate.rs task path, not standalone PRD |
| R4_D02 withhold router rewards | SOLID | Same limitation as D01 |
| R4_E01 repo context tests | SOLID | 20+ tests including live-repo integration |
| R4_E02 plan validation tests | SOLID | `include!` approach but substantive edge cases |

### R5: Telemetry & Learning (11/21 committed)

| Task | Rating | Key Finding |
|------|--------|-------------|
| R5_A01 parse usage | SOLID | — |
| R5_A02 UsageObservation type | SOLID | — |
| R5_A03 thread usage efficiency | SOLID | `cost_usd_without_cache` always equals `cost_usd` |
| R5_A04 thread usage episodes | SOLID | — |
| R5_A05 unknown cost display | SOLID | — |
| R5_C03 align learn all | SOLID | — |
| R5_C05 truthful projections | SOLID | StateHub vs disk staleness possible |
| R5_E01 usage unit test | SOLID | — |
| R5_E03 learn path test | SOLID | — |
| R5_F01 ACP episode logging | PARTIAL | Token/cost always zero; only wall_ms populated |
| R5_F05 knowledge card | SOLID | — |
| R5_F06 provenance cards | SOLID | — |

### R6: Security Posture (5/12 committed)

| Task | Rating | Key Finding |
|------|--------|-------------|
| R6_B01 terminal auth gate | SOLID | `acknowledge_public_risk` bypass not guarded |
| R6_B02 bearer token auth | SOLID | Auth is opt-in and off by default |
| R6_C02 share scrubbing | PARTIAL | Server-side works; CLI Gist path has zero scrubbing |
| R6_C03 share expiration | SOLID | No GC for expired files; weak share token entropy |
| R6_D02 negative proof test | SOLID | Missing tests for `acknowledge_public_risk` path |

### R7: Mori Polish + ACP (14/14 committed — "100%" but quality varies)

| Task | Rating | Key Finding |
|------|--------|-------------|
| R7_A01 /tools | SOLID | — |
| R7_A02 /mcp | SOLID | — |
| R7_A03 /context | PARTIAL | In chat_inline.rs not chat_session.rs; no tests |
| R7_A04 /history | PARTIAL | Same as A03 |
| R7_B01 rich tool display | SOLID | — |
| R7_B02 cost/token summary | SOLID | — |
| R7_C01 roko init --demo | SOLID | — |
| R7_C02 seed data badge | SOLID | — |
| R7_E03 MCP mesh polish | HOLLOW | **Runtime panic** on `roko config mcp` |
| R7_F01 ACP history | SOLID | — |
| R7_F02 file change notify | SOLID | — |
| R7_F03 slash commands + concurrency | SOLID | 50+ commands, CAS busy guard |
| R7_F04 phase badges | SOLID | — |
| R7_F05 narrative text | SOLID | — |
| R7_F06 context provider | PARTIAL | No registry pattern; hardcoded match arms |
| R7_F10 forensic gate failure | SOLID | — |

### D8: Demo & Bench (36 committed)

| Group | Rating | Key Finding |
|-------|--------|-------------|
| B: Dashboard Components | SOLID | Real API calls, SSE refetch, force-directed graph |
| C: Dashboard Wiring | SOLID | useLiveApi pages blank without server (by design) |
| D: Demo Scenarios | PARTIAL | Rust demo_seed.rs and TS demo-data.ts are independent; schema drift |
| E: Bench Infrastructure | PARTIAL | `run_task_real()` is a TODO stub (sleep + fake data) |
| F: Playbook/Mori-Diff | SOLID | All 5 FeedbackSinks wired into hot path; override-learning gap documented |

---

## Prioritized Fix List

### P0 — Must Fix (blocks users or security hole)
1. **R7_E03**: Wire `ConfigCmd::Mcp` dispatch in main.rs (runtime panic)
2. **Security**: Move share routes inside auth middleware (unauthenticated share creation)
3. **Security**: Auto-provision auth key on cloud deploy (wide-open stock deploys)

### P1 — Should Fix (feature doesn't work as claimed)
4. **R3 streaming drain**: Forward events to TUI instead of draining them silently
5. **R4_A06**: Wire `build_repo_context` into `plan generate` and `plan regenerate`
6. **R4_C05**: Inject validation diagnostics into regeneration prompt
7. **R2_G01**: Read pipeline template from config instead of hardcoding "standard"
8. **R6_C02**: Add scrubbing to CLI Gist path (`share.rs`)

### P2 — Should Fix (correctness/quality)
9. **R2_C02**: Remove `gate_for_name("shell")` stub or thread config through
10. **R2_B05**: Unify orchestrate.rs per-task model selection with model_selection.rs
11. **R4_C02**: Normalize model aliases at execution time, not just validation
12. **R5_F01**: Thread provider usage through ACP streaming path (zero-token episodes)
13. **R4_B02**: Make grounding section validation blocking, not advisory
14. **Config schema split**: Make init emit `[gates]` struct format matching runtime

### P3 — Cleanup
15. **R3_E01**: Scaffold a real `send_turn_api` stub with interface contract
16. **R2_D02**: Replace global atomic flag with Result-based exit
17. **D8_D**: Synchronize demo_seed.rs and demo-data.ts schemas
18. **D8_E**: Wire `run_task_real()` to actual dispatch (bench demo CLI)
19. **Deduplicate**: `resolve_effective_model_key()` across prd.rs/plan.rs
