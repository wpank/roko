# Mega-Parity Audit: All 113 Completed Tasks (Deep Audit v2)

**Date:** 2026-04-29
**Branch:** wp-arch2
**Scope:** 77 R2-R7 tasks + 36 D8 tasks committed via mega-parity runners
**Method:** 14 parallel agent audits reading actual source code, tracing call paths, verifying runtime reachability

---

## Executive Summary

| Verdict | Count | Notes |
|---------|-------|-------|
| FULLY_DONE and wired | 58 | Code exists, is called, works as intended |
| PARTIAL (real code, gap in wiring) | 18 | Code exists but missing a path, has wrong data, or only covers one surface |
| DEAD CODE (exists, zero callers) | 5 | R3_A01-A03, R4_A01, R7_F06 |
| STUB / MISSING | 2 | R7_F06 (entirely absent), bench demo --real |
| ANTI_PATTERN | 1 | bench strategies (corrected — see note below) |
| OVERCLAIMED | 3 | D8_C01, R2_G01, R7_C02 |

**Correction from v1:** The bench strategies (D8_F05-F09) are NOT label-only. Deep audit found they DO produce measurably different behavior:
- `Minimal` → skips learning stores, no playbook injection, no anti-pattern recording
- `ContextEnriched` → injects learned playbooks into system prompt
- `NeuroAugmented` → injects playbooks + neuro knowledge entries
- `FullCascade` → same as NeuroAugmented (these two are still identical)

So 3 of 4 are distinct; only FullCascade = NeuroAugmented.

**Critical issues found: 16** (expanded from 10 in v1)

---

## Critical Issues (ordered by severity)

### SECURITY

**1. CLI share path has zero secret scrubbing**
- **Task:** R6_C02 | **File:** `crates/roko-cli/src/share.rs`
- `share_run()` posts raw unscrubbed transcripts to public GitHub Gists. The `LogScrubber` exists in `roko-serve` but is never applied to the CLI path.

### SILENT DATA LOSS

**2. `[[task.verify]]` commands silently ignored in V2 path**
- **Task:** R2_C03 | **File:** `crates/roko-cli/src/orchestrate.rs`
- When `roko plan run` executes via the V2 WorkflowEngine, per-task `verify = ["cargo check"]` declarations in tasks.toml are **silently discarded**. GateService only runs gates from `roko.toml`'s `[gates]` section — it knows nothing about per-task verify steps. The PlanRunner state machine (Path B) does handle them, but it is not the default path.

**3. `[[gate]]` arrays from `roko init --profile rust` silently discarded by `roko plan run`**
- **Task:** R2_A05 | **File:** `crates/roko-cli/src/commands/init.rs`
- `roko init --profile rust` emits `[[gate]]` TOML sections with `cargo check/test/clippy`. The CLI-layer `Config` reads these correctly (so `roko run` works). But `RokoConfig::from_toml()` (used by `roko plan run`) silently ignores `[[gate]]` — it uses `[gates]` with `GatesConfig` struct (booleans like `clippy_enabled`, `skip_tests`). **Gates fire for `roko run` but are silently lost for `roko plan run`.**

**4. Episode path mismatch — `roko learn episodes` always shows "No data"**
- **Task:** R2_E02 | **File:** `crates/roko-cli/src/commands/learn.rs:394`
- `learn_episodes_path()` returns `.roko/learn/episodes.jsonl` but `EpisodeLogger` writes to `.roko/episodes.jsonl`.

**5. Double-write to `episodes.jsonl`**
- **Scope:** mori-diffs wiring
- `EpisodeSink` (new path) and `emit_feedback()` (legacy path) both append to the same `episodes.jsonl`. The legacy path writes richer data (actual tokens/cost). The new path writes zeroed `AgentOutcome` data. Result: redundant entries per task with inconsistent quality.

### BROKEN FEATURES

**6. Skipped verdicts trigger `GateFailed` in EffectDriver**
- **Task:** R2_C04 | **File:** `crates/roko-gate/src/gate_service.rs`
- `GateReport::first_failure()` returns skipped gates as failures. In `EffectDriver::run_gates`, this triggers `GateFailed` for a fully-skipped gate set, sending the agent to auto-fix when nothing actually failed.

**7. Seed data badge never appears (R7_C02)**
- **Task:** R7_C02 | **Files:** `useApiWithFallback.ts`, `projection_contract.rs`
- `roko init --demo` writes `source: "seed"` markers. But the serve layer strips this marker: episode API returns `"source": "episode_log"`, efficiency returns no `source` field. The `deriveDataMode()` function requires 100% seed items — impossible with current serve-side code. Badge never renders.

**8. V2 WorkflowEngine has no adaptive gate thresholds**
- **Task:** R2_C06-related | **File:** `crates/roko-orchestrator/src/service_factory.rs:195`
- `GateService::new()` is constructed without `with_adaptive_thresholds`. Every V2 run starts fresh with no EMA data. The PlanRunner state machine (Path B) does wire them.

**9. Gate verdicts not logged to `episodes.jsonl` in V2 path**
- **Scope:** gate truth pipeline
- In the V2 WorkflowEngine path, verdicts go to `EffectDriver::record_gate_verdict` (internal feedback sink), NOT to `EpisodeLogger`. Gate data is missing from the episode log for default runs.

### DATA QUALITY

**10. Cost precision loss at agent boundary**
- **Task:** R5_A01 | **File:** `crates/roko-agent/src/claude_cli_agent.rs:444`
- `usage_from_stream()` collapses `None` (no result event) to `0.0f32`. The "unknown vs zero" distinction that `UsageObservation` was designed to carry is silently destroyed.

**11. `roko status` never shows cost (hardcoded None)**
- **Task:** R5_A05 | **File:** `crates/roko-cli/src/status.rs:209-210`
- `collect_session_status_with_process_ledger()` hardcodes `total_cost_usd: None`. The efficiency log is never consulted.

**12. `AgentOutcome` is zeroed in FeedbackFacade events**
- **Scope:** mori-diffs runtime_feedback
- `runner_event_to_feedback()` constructs `AgentOutcome` with `model: ""`, `tokens_in: 0`, `cost: 0`. This makes `RoutingObservationSink` a no-op (empty model ignored by CascadeRouter) and `KnowledgeIngestionSink` writes useless candidates.

### DEAD CODE

**13. `ChatAgentSession` has zero callers**
- **Tasks:** R3_A01, R3_A02, R3_A03
- The entire type and all methods are never called. `chat_inline.rs` uses its own `ChatSession`. Also has a latent `serde_yaml` compile error (crate not in Cargo.toml, masked by `#![allow(dead_code)]`).

**14. `RepoContextPack` has zero callers**
- **Tasks:** R4_A01, R4_A02
- Good struct with tests but never constructed or used by `plan_generate.rs`, `prd.rs`, or `orchestrate.rs`.

### MISSING FEATURES

**15. R7_F06 context provider registry does not exist**
- **Task:** R7_F06 | **Files:** `crates/roko-acp/src/session.rs`, `bridge_events.rs`
- No `ContextProvider`, `ProviderRegistry`, or `register_context` exists anywhere. What exists instead is hardcoded two-source context resolution (neuro + file mentions).

**16. `roko bench demo --real` is a simulation stub**
- **Task:** D8_F01-F04 | **File:** `crates/roko-cli/src/bench_demo.rs:547-552`
- `run_task_real()` has a TODO and calls `simulate_task()` with 500ms delay. Note: the web UI bench path (`POST /api/bench/run`) IS real — this only affects the CLI `roko bench demo` command.

---

## Structural Findings (from deep audit)

### Two Parallel Model Selection Paths

The `EffectiveModelSelection` module (R2_B02) is NOT the single source of truth.

| Path | Uses module? | Uses CascadeRouter? | When |
|------|-------------|---------------------|------|
| `roko prd draft/plan` | Yes | No | CLI commands |
| `roko plan generate/regenerate` | Yes | No | CLI commands |
| `roko run` (v2/legacy with --model) | Yes | No | CLI with override |
| `roko run` (legacy, no --model) | No | No | Falls to `config.agent.model` |
| `roko plan run` (normal task) | **No** | Yes (inline 8-step) | The main hot path |
| `roko plan run` (hard override) | Yes (validation only) | No | Rare case |
| `roko prd consolidate` | **No** | No | Bypasses entirely |
| `roko chat` | Yes | No | Interactive |

The `plan run` hot path has its own inline 8-step selection: `task_hint → force_override → role_routing → cascade_router → lookahead → budget_guardrail → provider_health → daimon`. This is structurally independent from `EffectiveModelSelection`.

### Two Parallel Gate Execution Paths

| Path | Gate source | Per-task verify? | Adaptive thresholds? | Episode logging? |
|------|-----------|-----------------|---------------------|-----------------|
| **V2 WorkflowEngine** (default) | `roko.toml [gates]` | No — silently ignored | No | No (goes to EffectDriver) |
| **PlanRunner state machine** | `roko.toml` + `[[gate]]` | Yes | Yes | Yes |

The V2 path is default for `roko plan run`. The PlanRunner state machine is used by legacy code paths. This means per-task verify steps and adaptive thresholds are dead in the default flow.

### Two Parallel Playbook Extraction Paths

| Path | Source of playbook | When |
|------|-------------------|------|
| **Bench (roko serve)** | `extract_playbook_from_episode` — actual agent tool-call sequences | `POST /api/bench/run` |
| **Plan executor (orchestrate.rs)** | `build_task_playbook` — static task TOML metadata (title, description, files, verify) | `roko plan run` |

The plan executor creates playbooks from what was *planned*, not what the agent *actually did*. `extract_playbook_from_episode` (D8_F11) is never called from the plan run hot path.

### ACP Legacy vs Default Path Split

R7_F02 (file changes), R7_F04 (phase badges), R7_F05 (narrative), and R7_F10 (forensic analysis) only work in the **legacy** `run_workflow_pipeline` path (gated behind `ROKO_ACP_LEGACY` env var). The default `run_with_workflow_engine` path doesn't emit these.

### Mori-Diffs Modules: On Hot Path, But With Gaps

| Module | On hot path? | Key gap |
|--------|-------------|---------|
| `dispatch/Dispatcher::plan()` | Yes — per task | `Dispatcher::dispatch()` (bridge) only in tests |
| `dispatch/spawn_streaming_cli_agent()` | Yes — per task | — |
| `runtime_feedback/FeedbackFacade` | Yes — fire-and-forget | `AgentOutcome` is zeroed (model="", tokens=0) |
| `runtime_feedback/EpisodeSink` | Yes | Double-writes with legacy path |
| `projection/Projection::publish()` | Yes | **No subscribers** — events broadcast into void |
| `projection/DashboardProjection` | **Never instantiated** | Built but not activated |
| `projection/CliProgressPrinter` | **Never instantiated** | Built but not activated |
| `runner/persist.rs` | Yes | All functions used |
| `runner/resume.rs/prepare_resume()` | Yes | Called unconditionally at startup |

---

## Per-Task Audit Detail

### Runner 2: execution-contract (20/32 committed)

| Task | Status | Quality | Key Issue |
|------|--------|---------|-----------|
| R2_A02 | FULLY_DONE | acceptable | `detect_init_profile()` re-parses argv; no round-trip parse test |
| R2_A03 | FULLY_DONE | good | — |
| R2_A05 | **BROKEN for plan run** | poor | `[[gate]]` silently discarded by `RokoConfig`; works for `roko run` only |
| R2_A06 | PARTIAL | acceptable | Gate format validation blind (parses via `RokoConfig` which discards `[[gate]]`) |
| R2_B02 | FULLY_DONE | good | 6-step precedence chain, 8 tests |
| R2_B03 | FULLY_DONE | acceptable | `workflow_template` hardcoded to `"standard"` in v2 path |
| R2_B04 | FULLY_DONE | good | — |
| R2_B05 | PARTIAL | acceptable | Module used for validation only; normal CascadeRouter path bypasses it |
| R2_B06 | FULLY_DONE | acceptable | — |
| R2_C02 | PARTIAL | poor | `gate_for_name("shell")` returns `true` stub; runtime works via different path |
| R2_C03 | FULLY_DONE | good | Shell gate config→dispatch is correct; but per-task verify ignored in V2 |
| R2_C04 | PARTIAL | acceptable | `first_failure()` includes skipped gates → triggers GateFailed incorrectly |
| R2_C05 | FULLY_DONE | good | Skipped excluded from pass rate and learning |
| R2_C06 | FULLY_DONE | good | Tests real GateService; doesn't cover V2 WorkflowEngine integration |
| R2_D02 | FULLY_DONE | acceptable | Exit relies on call order of `find_topic`/`topic_names` |
| R2_E02 | **PATH BUG** | poor | Episode read path ≠ write path |
| R2_E03 | FULLY_DONE | acceptable | Test validates wrong path (matches the buggy read) |
| R2_F01 | FULLY_DONE | good | — |
| R2_F02 | FULLY_DONE | good | JSON 404 with 3 integration tests |
| R2_G01 | **OVERCLAIMED** | poor | `[pipeline]` config only wired in inline `roko run` TTY path; ignored by `plan run` and `run --engine v2` |

### Runner 3: agent-session-parity (3/29 committed)

| Task | Status | Quality | Key Issue |
|------|--------|---------|-----------|
| R3_A01 | DEAD CODE | acceptable design | Zero callers; `chat_inline.rs` uses own `ChatSession` |
| R3_A02 | DEAD CODE | good impl | Only reachable via R3_A01's dead constructor |
| R3_A03 | DEAD CODE + BUG | poor | `serde_yaml` not in Cargo.toml (latent compile error); zero callers |

### Runner 4: plan-grounding (2/24 committed)

| Task | Status | Quality | Key Issue |
|------|--------|---------|-----------|
| R4_A01 | DEAD CODE | good design | Zero callers; no constructor; never injected into PRD/plan gen |
| R4_A02 | FULLY_DONE (within scope) | good | Solid Rust/TS/Go/Python detection; tests pass; but zero callers |

### Runner 5: telemetry-learning (4/27 committed)

| Task | Status | Quality | Key Issue |
|------|--------|---------|-----------|
| R5_A01 | FULLY_DONE | acceptable | `None` cost → `0.0f32` loses unknown-vs-zero distinction |
| R5_A02 | FULLY_DONE | acceptable | Conversion round-trips are lossy; no tests in file |
| R5_A05 | PARTIAL | acceptable | Only `roko learn` got the fix; `roko status` and serve routes still show nothing |
| R5_E01 | FULLY_DONE | good | 4 unit + 2 integration tests |

### Runner 6: security-posture (3/12 committed)

| Task | Status | Quality | Key Issue |
|------|--------|---------|-----------|
| R6_B01 | FULLY_DONE | good | Three-layer terminal auth policy, 3 tests |
| R6_C02 | **PARTIAL (security)** | poor | Serve-side scrubbing works; CLI Gist path is unprotected |
| R6_C03 | FULLY_DONE (serve) | good | TTL, HTTP 410, configurable; CLI Gists have no expiry |

### Runner 7: mori-polish (9/14 committed)

| Task | Status | Quality | Key Issue |
|------|--------|---------|-----------|
| R7_C01 | FULLY_DONE | good | Realistic seeding with `source: "seed"` markers |
| R7_C02 | **BROKEN** | poor | Badge never appears; server strips `source` marker from episodes |
| R7_F01 | FULLY_DONE | good | Dual-trim history, API+CLI injection paths |
| R7_F02 | PARTIAL | acceptable | Only in legacy pipeline path; default path doesn't emit file changes |
| R7_F03 | PARTIAL | acceptable | Slash commands real (spawn subprocesses); `Arc<RwLock>` not done |
| R7_F04 | FULLY_DONE (legacy only) | good | All PipelinePhase variants covered; only in legacy runner path |
| R7_F05 | FULLY_DONE (legacy only) | good | Context-aware narratives; only in legacy runner path |
| R7_F06 | **MISSING** | N/A | No context provider registry exists |
| R7_F10 | PARTIAL | acceptable | Real classifier + weak episode matching; only in legacy runner path |

### D8: demo/bench (36/36 committed)

| Group | Status | Key Finding |
|-------|--------|-------------|
| D8_B01-B07 (dashboard widgets) | FULLY_DONE | All 7 are real Canvas 2D / React components; use `useLiveApi` (no fallback); real API endpoints |
| D8_C01 (learning state load) | OVERCLAIMED | Loading is lazy per-request, not eager at startup as described |
| D8_C02 (knowledge usage feedback) | FULLY_DONE | Real — `record_knowledge_usage_feedback` called on verify paths |
| D8_C05 (gateway inference events) | FULLY_DONE | Real — publishes to event bus + durable JSONL |
| D8_D01-D05 (demo scenarios) | FULLY_DONE | All use real PTY commands, not scripted playback; D8_D04 (gate retry) is strongest |
| D8_E01 (bench end-to-end) | PARTIAL | Web UI bench (`POST /api/bench/run`) is real dispatch; CLI `bench demo --real` is stub |
| D8_E02 (model comparison) | FULLY_DONE | Matrix dispatches real runs; offline falls back to demo data |
| D8_E03 (cost attribution) | PARTIAL | Real data when live; `gate_verdicts` field always empty in real runs |
| D8_E04 (export/import) | PARTIAL | Export real (server endpoint); import client-only (lost on reload) |
| D8_F01 (learnable-rust suite) | FULLY_DONE | 5 real tasks with specific prompts and gate criteria |
| D8_F02 (Cerebras model) | FULLY_DONE | Real backend with full adapter; list appearance is config-dependent |
| D8_F03 (workdir scaffold) | FULLY_DONE | Creates real compilable Cargo project |
| D8_F04 (usage from dispatch) | PARTIAL | Populated from API backends; CLI backend may yield None → zeroed |
| D8_F05-F06 (strategy enum) | FULLY_DONE | Real enum, threaded through API and dispatch |
| D8_F07 (Minimal strategy) | FULLY_DONE | No-op baseline, skips all learning |
| D8_F08 (ContextEnriched) | FULLY_DONE | Injects learned playbooks into system prompt |
| D8_F09 (NeuroAugmented) | FULLY_DONE | Playbooks + neuro knowledge (FullCascade is identical to this) |
| D8_F11-F14 (playbook extraction) | FULLY_DONE (bench only) | Real extraction from tool-call sequences; NOT called from plan run |
| D8_F15-F17 (anti-pattern extraction) | FULLY_DONE (bench only) | Real extraction with dedup; plan executor uses different inline mechanism |
| D8_F19 (BenchLearningEvent SSE) | FULLY_DONE | Real; only fires for non-Minimal strategies |

---

## Demo App Assessment

**Verdict: Real dashboard, not a fake demo.** All 17 routes render real UI. Dashboard pages use `useLiveApi` (no silent fallback). When `roko serve` is down, pages freeze at last-known state rather than switching to demo data.

The `useApiWithFallback` hook with demo data exists but **dashboard pages were migrated off it** during mega-parity. It only remains for non-dashboard paths. The fallback is completely silent (no toast/banner when serving fake data) — a UX concern if it's ever re-enabled.

---

## roko-serve Route Count

Claimed: ~85. **Actual: ~170 logical endpoints, ~223 registered paths** (dual `/learning/*` + `/learn/*` aliases inflate the count). All route handlers are real — no stubs, no production panics (all `panic!` calls confirmed inside `#[cfg(test)]` blocks).

---

## Anti-Pattern Summary

### 1. V2 vs PlanRunner path split (systemic)
The codebase has two complete execution engines with different feature sets. The V2 WorkflowEngine is the default but is missing: per-task verify steps, adaptive thresholds, episode gate logging, ACP narrative/badges/forensics. The PlanRunner state machine has all of these but is not the default path.

### 2. Built but never called (5 tasks)
R3_A01-A03, R4_A01, R7_F06. Crate-level `#![allow(dead_code)]` masks warnings.

### 3. Two parallel playbook mechanisms
Bench uses agent tool-call extraction (good). Plan executor uses task TOML metadata (captures plan intent, not observed behavior).

### 4. Two parallel model selection pipelines
`EffectiveModelSelection` module for CLI commands. Inline 8-step pipeline for `plan run`. They can disagree.

### 5. Legacy-gated ACP features
R7_F02, F04, F05, F10 only work behind `ROKO_ACP_LEGACY` env var.

### 6. Config schema split
CLI `Config` reads `[[gate]]`. `RokoConfig` reads `[gates]`. `roko init` writes `[[gate]]`. `roko plan run` uses `RokoConfig`. Result: init-generated gates are invisible to plan run.

---

## Recommendations

### Immediate (before next merge to main)
1. Fix R6_C02 CLI scrubbing (security)
2. Fix R2_E02 episode path mismatch
3. Fix skipped-verdict-triggers-GateFailed bug (R2_C04)
4. Remove `--real` flag from `bench demo` or wire it

### High priority (next sprint)
5. Decide V2 vs PlanRunner: either migrate V2 to support per-task verify + adaptive thresholds, or make PlanRunner the default
6. Unify `[[gate]]` and `[gates]` config schemas so `roko init` output is compatible with `roko plan run`
7. Fix `source: "seed"` marker flow through serve layer (R7_C02)
8. Stop double-writing episodes; pick one path and give it real `AgentOutcome` data
9. Wire `ChatAgentSession` into `chat_inline.rs` or delete it
10. Wire `RepoContextPack` into plan generation or delete it

### Cleanup
11. Remove `#![allow(dead_code)]` from `roko-cli/src/lib.rs`
12. Activate `DashboardProjection`/`CliProgressPrinter` subscribers during plan run
13. Plumb `AgentOutcome` from dispatch layer into `RunnerEvent::TaskAttemptCompleted`
14. Merge `FullCascade` with `NeuroAugmented` or differentiate them
15. Mark R7_F06 as NOT_DONE
16. Update route count claim from "~85" to "~170"
