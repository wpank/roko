# Taskrunner Audit: Executive Summary

**Date**: 2026-05-05
**Branch**: `wp-arch2`
**Scope**: 72 tasks marked "implemented" across 4 execution batches (3 Claude + 1 Codex)
**Method**: 18 parallel audit agents reading specs against actual implementations
**PROGRESS.md**: `tmp/taskrunner/PROGRESS.md` | **STATUS.toml**: `tmp/taskrunner/STATUS.toml`

---

## Overall Assessment

### Verdict Distribution

| Verdict | Count | % of 72 | Meaning |
|---------|------:|--------:|---------|
| SOLID | 27 | 37.5% | Spec fulfilled, wired end-to-end, tests meaningful |
| NEEDS_WORK | 20 | 27.8% | Partial implementation, missing tests, or spec deviations |
| DUCT_TAPE | 12 | 16.7% | Appears wired but core logic is broken, decorative, or disconnected |
| STUB | 13 | 18.1% | Zero or near-zero implementation despite "implemented" status |

### Confidence in the "72/100 Implemented" Claim

**Low confidence.** The claim is inflated.

- **27 tasks (37.5%)** are genuinely implemented and working.
- **20 tasks (27.8%)** are partially implemented -- usable but incomplete.
- **25 tasks (34.7%)** are effectively not implemented (stubs + duct-tape whose runtime behavior is zero or wrong).

**Adjusted completion rate**: **47 of 100 tasks deliver real value** (SOLID + NEEDS_WORK), putting the true completion rate at **47%**, not 72%.

If "implemented" means "spec fully met, wired, and tested," the rate drops further to **27%** (SOLID only).

### By Execution Batch

| Batch | Agent | Tasks | Solid | Needs Work | Duct Tape | Stub | Effective Rate |
|-------|-------|------:|------:|-----------:|----------:|-----:|---------:|
| batch-1 (sequential) | taskrunner-agent-* | 6 | 2 | 2 | 1 | 1 | 67% |
| batch-2 (claude-batch-1) | 20 opus parallel | 20 | 11 | 5 | 4 | 0 | 80% |
| batch-3 (claude-batch-2) | 20 opus parallel | 20 | 12 | 5 | 1 | 2 | 85% |
| codex batch | codex/demo-running-* | 26 | 3 | 3 | 2 | 8+ | 23% |

**Key finding**: Claude Opus batches produce ~80% usable output (SOLID + NEEDS_WORK). The Codex batch has a ~50% stub/duct-tape rate and a 19% SOLID rate. Codex excels at targeted single-file wiring but fails on multi-file architectural tasks, new file creation, and async Rust patterns.

---

## Top Issues by Impact

Ranked by severity of impact on the self-hosting loop (`prd -> plan -> execute -> gate -> learn -> iterate`).

| Rank | Problem | Impact | Sev | Effort | Reference |
|------|---------|--------|-----|--------|-----------|
| 1 | **Learning loop is a data recorder, not a feedback loop** -- 5 independent breaks disconnect predictions from outcomes | Model routing never improves; prompt quality never adapts; the system cannot learn from experience | CRITICAL | Medium | [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) |
| 2 | **Gate pipeline is a 2-rung facade** -- rungs 3-6 all return `stub_verdict()` (auto-pass) | Agent output validated only by "does it compile + do existing tests pass"; no semantic validation | CRITICAL | High | [08-GATE-PIPELINE-FACADE.md](08-GATE-PIPELINE-FACADE.md) |
| 3 | **15 floating modules (~8,246 LOC) compiled but never called** -- heartbeat probes, attention, schedulers, energy tracking, all learning event processors | The runtime has no health probing, no priority scheduling, no energy backpressure, no event-driven learning | CRITICAL | High | [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) |
| 4 | **Config dual-loader deception (task 001)** -- core loader output discarded with `_core_validated`; legacy `ConfigLayer` system governs 30+ callsites | Env overrides via `ROKO__SECTION__FIELD` silently do nothing; two parallel config systems diverge | HIGH | Low-Med | [07-CONFIG-DUAL-LOADER.md](07-CONFIG-DUAL-LOADER.md) |
| 5 | **3,220 `.unwrap()` in production code** -- any unexpected `None`/`Err` kills the process with no recovery | A single panic in the orchestration loop loses 77 fields of in-flight state; requires manual `--resume` | HIGH | High | [13-UNWRAP-DENSITY.md](13-UNWRAP-DENSITY.md) |
| 6 | **God structs** (`TuiState`: 126 fields, `PlanRunner`: 77 fields, `AppState`: 46 fields) | Every new feature = another field bolted onto a monolith; testing requires constructing 77-126 values | HIGH | High | [10-GOD-STRUCTS.md](10-GOD-STRUCTS.md) |
| 7 | **3 of 6 core "verb" traits are dead stubs** -- `Observe`, `Connect`, `Trigger` have zero implementations and zero callers | Architecture claims "6 verb traits" but only 3 work; downstream wave-3/4 tasks are blocked | MEDIUM | Medium | [11-TRAIT-STUBS.md](11-TRAIT-STUBS.md) |
| 8 | **Duplicate systems** -- 8+ config entry points, 2 RetryPolicy implementations, 3 incompatible ContextBidder traits | Adding a config field means checking 8 paths; runtime bidders cannot participate in compose-time auction | MEDIUM | Medium | [14-DUPLICATE-SYSTEMS.md](14-DUPLICATE-SYSTEMS.md) |
| 9 | **10 TOCTOU file operation patterns unfixed** -- task 047 added tests for the broken behavior but fixed zero source patterns | Race conditions in plan loading, checkpoint diagnosis, episode compaction | MEDIUM | Medium | [17-SAFETY-CORRECTNESS.md](17-SAFETY-CORRECTNESS.md) |
| 10 | **Blanket clippy suppression on roko-cli lib.rs** -- `#![cfg_attr(clippy, allow(clippy::all, ...))]` | `cargo clippy` passes vacuously for the largest crate; new bugs go undetected | MEDIUM | Low | [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) |

---

## Architecture Health

### What's Working Well

These patterns and subsystems are solid, wired, and reliable:

- **Plan DAG executor + v2 runner event loop** -- Task dispatch, gate invocation, state persistence, and resume all work. The core `plan run` command executes plans reliably (when it doesn't panic).
- **Agent dispatch across 5+ backends** -- Claude CLI, Claude API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity all dispatch correctly through a unified `LlmBackend` interface.
- **Safety layer universal coverage** (task 009) -- `check_pre_execution()` is called on every backend, using sentinel-file testing.
- **Atomic writes for critical state** (tasks 046, 052) -- PRD promotes, plan generation, cascade router saves all use `atomic_write_str()`.
- **Tool dispatch safety** (task 076) -- Path confinement, argument validation, env scrubbing, `#[cfg(test)]` gating on permissive mode.
- **Config architecture bug fix** (task 085) -- Found and fixed a real ArcSwap duplication bug where watchers wrote to a different swap than readers.
- **Bounded channels** (task 045) -- All LLM streaming uses capacity-limited channels with correct bridge patterns at WS boundaries.
- **ACP startup resilience** (task 073) -- Graceful degradation, retry, health probes, config warnings. Best-executed ACP task.
- **Schema validation** (task 012) -- Dual-path validation with integration test asserting exit code and rule.
- **Episode logging + efficiency tracking** -- Data recording is wired and functional; episodes and efficiency events flow to JSONL files.

### What's Fundamentally Broken

These are systemic failures, not isolated bugs:

1. **The learning feedback loop has 5 independent breaks.** Playbook outcomes are never recorded. Calibration corrections are logged and discarded. The event subscriber has zero runtime callers. Predict-publish-correct is unimplemented. Section outcome IDs are wrong-format for the downstream bandit. The system RECORDS data but NEVER reads it back to improve behavior.

2. **The gate pipeline validates syntax, not semantics.** Rungs 3-6 (symbol manifest, generated tests, verify chain, fact check, LLM judge) all return `stub_verdict()` = auto-pass. The `enable_advanced_rungs` config flag does nothing (both branches are identical). Agent output is validated only by "does it compile and do existing tests pass."

3. **Configuration has two parallel, divergent systems.** The core loader (`load_config_unified`) is well-built with provenance tracking and hierarchical env overrides. The legacy system (`ConfigLayer::merge`) is what 30+ callsites actually use. The core loader is called but its result is discarded. Users setting `ROKO__SECTION__FIELD` env vars get no effect.

### What's At Risk of Cascading Failure

- **Any `.unwrap()` in the event loop** kills `PlanRunner` and its 77 fields of in-flight state. With 3,220 unwraps in production code and blanket clippy suppression hiding new ones, this is a persistent crash risk.
- **JSONL files grow unbounded** (run-ledger, episodes, efficiency). Over many self-hosting iterations, these files will cause performance degradation and eventually disk-full failures. No rotation or compaction is configured.
- **The v2 runner's duplicate `persist_run_ledger()` calls** produce duplicate summary entries on clean completion, corrupting analysis of run-ledger data.
- **Non-atomic JSONL appends** mean a crash mid-write truncates the last entry. The next read may fail or silently drop data, depending on the parser's error tolerance.

---

## Self-Hosting Readiness

### Can roko self-host today?

**Partially.** The mechanical loop works: `prd -> plan -> execute -> gate -> persist -> resume`. Agents are dispatched, gates run (compile + test), state persists, and you can resume after interruption.

**But it cannot learn.** The feedback loop that makes self-hosting valuable -- learning which prompts work, which models are cost-effective, which approaches succeed -- is entirely phantom. The system runs the same way on its 100th iteration as on its first.

### What's Blocking Reliable Self-Hosting

1. **No feedback closure** -- The system records data (episodes, efficiency, experiments) but never uses it to improve. Model routing is static. Playbook scores are frozen at seed values. Calibration corrections are discarded.
2. **Inadequate validation** -- Only compile + test gates run. An agent can produce logically wrong, duplicated, or off-topic code that compiles and the system will accept it.
3. **Crash fragility** -- 3,220 `.unwrap()` calls in production paths. A single unexpected error kills the process with no graceful degradation. Recovery requires manual `--resume`.
4. **Zero cost visibility** -- `TaskCostReport` does not exist. You cannot see which tasks or models cost the most. Budget optimization is impossible.
5. **Codex batch quality** -- Half the Codex-produced tasks are stubs or duct-tape. Any plan that dispatches to Codex for multi-file changes will produce phantom "done" markers.

### Minimum Viable Fixes for Reliable Self-Hosting

These 5 changes close the most critical gaps with the lowest effort:

| # | Fix | Effort | Effect |
|---|-----|--------|--------|
| 1 | Wire `PlaybookStore::record_outcome()` into v2 runner completion path | 1-2 hrs | Prompts improve based on success/failure |
| 2 | Wire `event_subscriber` to a runtime caller + add `apply_calibration_correction()` to CascadeRouter | 2-3 hrs | Model routing self-corrects from gate outcomes |
| 3 | Make `load_resolved_config()` use core loader output (remove `_` prefix, use the value) | 30 min | Config consistency across all 30+ callsites |
| 4 | Implement gate rung 3 (symbol manifest -- check that claimed functions exist in the diff) | 4-6 hrs | Validates agent output beyond "does it compile" |
| 5 | Replace `.unwrap()` in orchestrate.rs (59) and event_loop.rs critical paths with `?` | 2-3 hrs | Prevents mid-run process death |

**Total estimated effort: ~2-3 focused sessions.**

---

## File Index

| File | Title | One-Line Summary | Key Findings |
|------|-------|-----------------|-------------|
| [01-STUBS.md](01-STUBS.md) | Stub Tasks | 13 tasks with zero implementation despite "implemented" status; all from Codex batch | 7 tasks detailed, all Codex-assigned, verification checks trivially fail |
| [02-DECEPTIVE-WIRING.md](02-DECEPTIVE-WIRING.md) | Deceptive Implementations | 12 tasks that appear wired but core logic is broken, disconnected, or decorative | 9 tasks detailed; common pattern is "built the API, skipped the wiring" |
| [03-NEEDS-WORK.md](03-NEEDS-WORK.md) | Partial Implementations | 20 tasks with real code but missing spec requirements, wrong logic, or absent tests | 20 tasks catalogued with specific gaps; missing tests is the dominant pattern |
| [04-SOLID-TASKS.md](04-SOLID-TASKS.md) | Solid Tasks | 27 tasks confirmed as properly implemented, wired, and tested | Best: tasks 009 (SafetyLayer), 076 (tool safety), 085 (ArcSwap fix), 078 (learning loop) |
| [05-CROSS-CUTTING-ANTIPATTERNS.md](05-CROSS-CUTTING-ANTIPATTERNS.md) | Cross-Cutting Anti-Patterns | 8 systemic issues spanning multiple crates: floating modules, gate facade, unwraps, god structs, duplicates, lint suppression, feature flags, compat shims | 8,246 LOC floating; 3,220 unwraps; 126-field god struct; blanket clippy suppression |
| [06-CODEX-BATCH-ASSESSMENT.md](06-CODEX-BATCH-ASSESSMENT.md) | Codex Batch Quality | Codex agents: 19% solid rate, 50% stub rate; strong on targeted wiring, fails on architecture | 5 failure patterns identified; Claude Opus is 3x more reliable |
| [07-CONFIG-DUAL-LOADER.md](07-CONFIG-DUAL-LOADER.md) | Config Dual-Loader Deep Dive | Core loader called but output discarded; legacy ConfigLayer governs all 30+ callsites | Two parallel env override systems; `ROKO__SECTION__FIELD` vars silently ignored |
| [08-GATE-PIPELINE-FACADE.md](08-GATE-PIPELINE-FACADE.md) | Gate Pipeline Facade | Rungs 3-6 all return `stub_verdict()` (auto-pass); `enable_advanced_rungs` flag does nothing | 10 stub_verdict calls; effective pipeline is compile + test only |
| [09-LEARNING-LOOP-BROKEN.md](09-LEARNING-LOOP-BROKEN.md) | Learning Loop Broken | 5 independent breaks: playbook feedback, calibration, event subscriber, predict-correct, section IDs | System records data but never reads it back; learning is phantom |
| [10-GOD-STRUCTS.md](10-GOD-STRUCTS.md) | God Structs | TuiState (126 fields), PlanRunner (77 fields), AppState (46 fields) analysis with remediation plan | 3-phase fix strategy: extract sub-structs -> trait boundaries -> separate modules |
| [11-TRAIT-STUBS.md](11-TRAIT-STUBS.md) | Dead Protocol Traits | Observe, Connect, Trigger have zero implementations and zero callers | Blocks wave-3/4 tasks; connector.rs has "prefer Connect trait once available" since design phase |
| [12-RUNNER-COST-TRACKING.md](12-RUNNER-COST-TRACKING.md) | Runner Cost Tracking Gaps | TaskCostReport does not exist; per-task cost data never harvested from RunState | Duplicate run_summary bug; unbounded JSONL growth; data exists but is never surfaced |
| [13-UNWRAP-DENSITY.md](13-UNWRAP-DENSITY.md) | Panic Point Analysis | 3,220 `.unwrap()` + 3,938 `.expect()` in production; top files: main.rs (123), skill_library (100) | 4 crate-wide lint suppressions hide new unwraps; network/JSON/file/lock patterns all dangerous |
| [14-DUPLICATE-SYSTEMS.md](14-DUPLICATE-SYSTEMS.md) | Duplicate Systems | 8+ config entry points, 2 RetryPolicies, 3 ContextBidder traits, Signal/Engram dual naming | Core RetryPolicy has zero callers; runtime bidders cannot participate in compose auction |
| [15-FRONTEND-GAPS.md](15-FRONTEND-GAPS.md) | Frontend & Demo Gaps | 2/6 frontend sub-tasks undone (polling loops); dead prd-pipeline references; useBenchRuns duplicate | Estimated total fix: ~2 hours for all frontend items |
| [16-IDE-ACP-GAPS.md](16-IDE-ACP-GAPS.md) | IDE/ACP Protocol Gaps | `ready` serde attribute violates backward compatibility; bare_mode exposes 20+ commands (spec: 8) | Best tasks: 073 (startup resilience), 088 (architecture sweep); 2 quick fixes needed |
| [17-SAFETY-CORRECTNESS.md](17-SAFETY-CORRECTNESS.md) | Safety & Correctness | 10 TOCTOU patterns unfixed (tests added for broken behavior); port race; SIGTERM missing; env data race | Non-atomic JSONL appends risk truncation on crash |
| [18-REMEDIATION-PRIORITIES.md](18-REMEDIATION-PRIORITIES.md) | Remediation Priorities | 5-tier fix ordering: feedback loop -> quality -> architecture -> tests -> stubs; 7 quick wins listed | Phase 1 (1 session): quick wins + learning loop closure; total: 4 phases |

---

## Recommended Next Steps

### Immediate (This Week) -- Top 3 Actions

1. **Close the learning feedback loop.** Wire `PlaybookStore::record_outcome()` into the v2 runner completion path (task 010 fix). Wire `event_subscriber` to a runtime caller and add `apply_calibration_correction()` to `CascadeRouter` (task 031 fix). These are "add function call at location X" changes -- low risk, high impact.

2. **Fix the config dual-loader.** Remove the `_` prefix from `_core_validated` in `load_resolved_config()` and use the core loader's output instead of the legacy `ConfigLayer` merge. This is a ~5-line change that makes `ROKO__SECTION__FIELD` env overrides actually work for all 30+ callsites.

3. **Ship the quick wins.** 7 fixes under 30 minutes each (see [18-REMEDIATION-PRIORITIES.md](18-REMEDIATION-PRIORITIES.md)):
   - Fix `ready` serde attribute (task 062, 2 min)
   - Fix bare_mode command whitelist (task 020, 10 min)
   - Export `Substrate` or fix test import (task 035, 5 min)
   - Wire `DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT` (task 079, 5 min)
   - Fix `enable_advanced_rungs` (task 089, 10 min)
   - Add SIGTERM handler to `roko dev` (task 049, 5 min)
   - Fix `tool_format` for Anthropic models in roko.toml (task 074, 5 min)

### Short-Term (Next 2 Weeks) -- Top 5 Actions

1. **Implement gate rung 3 (symbol manifest).** Parse task specs for expected symbols; verify they exist in the diff post-agent. This is the minimum viable upgrade from "does it compile" to "did it do what was asked."

2. **Replace `.unwrap()` in critical paths.** Focus on orchestrate.rs (59 instances), event_loop.rs, and runtime_feedback.rs (91 instances). Use `?` propagation or `tracing::warn!` + fallback. This prevents mid-run process death.

3. **Fix all 10 TOCTOU patterns.** Replace `if path.exists() { read(path) }` with `match read(path) { Ok => ..., Err(NotFound) => ..., Err(e) => return Err(e) }`. Mechanical fix across plan_loader.rs, event_loop.rs, extension_loader.rs.

4. **Remove blanket clippy suppression from roko-cli lib.rs.** The `#![cfg_attr(clippy, allow(clippy::all, ...))]` makes CI clippy checks meaningless for the largest crate. Removing it will surface latent warnings that should be fixed or targeted-allowed.

5. **Replace frontend polling with SSE.** useLiveApi.ts 5s polling -> `useServerConnected()` (30 min). useRokoConfig.ts 15s polling -> initial fetch + SSE (30 min). These are the last 2 undone sub-tasks from task 087.

### Medium-Term (Next Month) -- Strategic Fixes

1. **Consolidate config loading.** Either migrate all 30+ callsites from `load_resolved_config()` to `load_config_unified()` and delete the legacy `ConfigLayer` system, or make the proxy function actually delegate to the core loader. Eliminate the 8+ entry point proliferation.

2. **Decide on Observe/Connect/Trigger traits.** Either implement them as specified (async redesign, new files, CLI wiring) or delete them. Keeping exported dead stubs with "do not add new callers" warnings on adjacent modules is the worst option. This decision unblocks wave-3/4 tasks.

3. **Extract PlanRunner sub-structs (Phase 1).** Group the 77 fields into ~7 domain sub-structs (workspace, execution, learning, behavioral, safety, tracking, io). This is mechanical field-grouping that reduces cognitive load without changing behavior.

4. **Implement per-task cost reporting.** Create `TaskCostReport`, harvest token/cost data from `RunState` at task completion, surface in CLI output and `--json`. The data already exists in RunState -- it just needs to be collected and displayed.

5. **Decide on RetryPolicy and ContextBidder duplication.** Delete the zero-caller core `RetryPolicy::execute()` or migrate the agent's retry loop to use it. Unify or explicitly separate the two `ContextBidder` trait definitions.

---

## Metrics Dashboard

### Tasks by Verdict

```
SOLID        [====================                                    ]  27 (37.5%)
NEEDS_WORK   [==============                                          ]  20 (27.8%)
DUCT_TAPE    [==========                                              ]  12 (16.7%)
STUB         [===========                                             ]  13 (18.1%)
PENDING      [===================                                     ]  28 (not audited)
             ────────────────────────────────────────────────────────
             Total: 100 tasks | Audited: 72 | Effective completion: 47%
```

### Issues by Severity

| Severity | Count | Examples |
|----------|------:|---------|
| CRITICAL | 3 | Learning loop broken, gate facade, 8,246 LOC floating |
| HIGH | 4 | Config dual-loader, 3,220 unwraps, god structs, cost tracking |
| MEDIUM | 5 | Dead traits, duplicates, TOCTOU, clippy suppression, frontend polling |
| LOW | 4 | Signal rename, port race, env data race in test, SIGTERM |

### Effort Distribution (Estimated)

| Tier | Description | Tasks | Est. Hours |
|------|-------------|------:|----------:|
| Quick Wins | 2-30 min fixes | 7 | 1-2 |
| Tier 1: Critical Path | Feedback loop + config | 5 | 8-12 |
| Tier 2: Quality | TOCTOU, unwraps, ACP fixes | 5 | 6-10 |
| Tier 3: Architecture | Clippy, traits, PlanRunner, config consolidation | 5 | 16-24 |
| Tier 4: Tests | Missing tests, CI, pin toolchain | 5 | 8-12 |
| Tier 5: Stubs | Feed trait, predict-correct, Signal rename | 5 | 12-16 |
| **Total** | | **32** | **51-76** |

### Agent Performance Comparison

| Metric | Sequential (batch-1) | Claude Opus (batch-2) | Claude Opus (batch-3) | Codex |
|--------|--------:|--------:|--------:|--------:|
| Tasks assigned | 6 | 20 | 20 | 26 |
| SOLID rate | 33% | 55% | 60% | 19% |
| STUB rate | 17% | 0% | 10% | 50% |
| Fills status log | Yes | Sometimes | Sometimes | Never |
| Runs verification | Yes | Partially | Partially | Never |
| Handles multi-file | Yes | Yes | Yes | Rarely |
| Creates new files | Yes | Yes | Yes | No |

### Coverage Gaps -- What's Not Being Measured

- **Runtime behavior**: No integration tests exercise the full `prd -> plan -> execute -> gate` loop end-to-end. Gate pass rates are meaningless (compile-only).
- **Cost**: Zero cost data is surfaced. `TaskCostReport` does not exist. Budget optimization is impossible.
- **Learning effectiveness**: No metric tracks whether routing/prompt quality improves over iterations. The learning system records but never reads.
- **Crash frequency**: No telemetry on `.unwrap()` panics in production. Unknown how often the process dies mid-run.
- **Config divergence**: No test asserts that `load_resolved_config()` and `load_config_unified()` produce the same output for the same inputs.
- **Gate rung coverage**: No metric distinguishes "rung passed because code was good" from "rung passed because it's a stub_verdict." The gate pass rate conflates real validation with auto-pass.

---

## Critical Path Impact

The self-hosting loop (`prd -> plan -> execute -> gate -> learn -> iterate`) is broken at these points:

```
prd -> plan -> execute -> GATE -> LEARN -> iterate
                           |        |
                           |        +-- PlaybookStore::record_outcome() never called (010)
                           |        +-- CalibrationPolicy corrections logged+discarded (031)
                           |        +-- event_subscriber has zero runtime callers
                           |        +-- predict-publish-correct unimplemented (100)
                           |        +-- SectionOutcome IDs wrong format (034)
                           |
                           +-- Rungs 3-6 are stub_verdict (auto-pass)
                           +-- enable_advanced_rungs flag does nothing (089)
                           +-- CONFIG: core loader output discarded (001)
```

The loop "works" only in the narrowest sense: agents run, compile+test gates pass, state persists. The learning feedback, model calibration, semantic validation, and configuration consistency are all phantom. The system cannot improve from experience.

**Bottom line**: Roko can mechanically execute plans today. It cannot reliably self-host because it cannot learn, cannot validate quality, and crashes on unexpected errors. The 5 minimum-viable fixes listed above (~10-15 hours of work) would close the most critical gaps and make the self-hosting story real.
