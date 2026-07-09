# D — Diagnosis Engine + Stuck Detection + Production Failure Catalog (Docs 04, 05, 14)

Parity audit of the three conductor docs that describe Roko's error-
analysis surface: the substring-match `DiagnosisEngine`, the 6-heuristic
`StuckDetector` + `MetaCognitionHook`, and the 21-entry production
failure catalog that cross-references both back to watchers / circuit
breaker / anomaly detector / `ProcessSupervisor`. Generated 2026-04-16.

---

## D.01 — Self-claim: `ErrorCategory` enum has 20 variants (Doc 4 §"Error Categories")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 4 §"Error Categories" (`04-diagnosis-engine.md:38-66`) enumerates exactly 20 `ErrorCategory` variants: `CompileError`, `TestFailure`, `TypeMismatch`, `BorrowCheckerError`, `LifetimeError`, `ImportError`, `MissingFile`, `PermissionDenied`, `NetworkError`, `TimeoutError`, `OomError`, `DiskFull`, `LlmRateLimit`, `LlmContextOverflow`, `LlmRefusal`, `ProcessCrash`, `LoopDetected`, `ClippyWarning`, `GitConflict`, `DependencyError`.
**Reality**: Exact match. `pub enum ErrorCategory` at `crates/roko-conductor/src/diagnosis.rs:23-67` has all 20 variants (counted from line `28` through `66`). Order in the source is slightly different from the doc (`ClippyWarning` / `GitConflict` / `DependencyError` appear early in the source, later in the doc) but the set is identical and `#[non_exhaustive]` marks the enum for forward compatibility. `#[serde(rename_all = "snake_case")]` ensures signal tags match the doc's "snake_case" convention.

---

## D.02 — Self-claim: `SuggestedIntervention` enum has 9 variants (Doc 4 §"Suggested Interventions")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 4 §"Suggested Interventions" (`04-diagnosis-engine.md:110-124`) enumerates exactly 9 interventions: `RetryWithContext`, `AutoFix`, `RestartAgent`, `AbortPlan`, `BackoffRetry`, `MergeResolution`, `ReduceContext`, `SwitchModel`, `WarnAndContinue`.
**Reality**: Exact match. `pub enum SuggestedIntervention` at `crates/roko-conductor/src/diagnosis.rs:72-94` has all 9 variants at lines `77-93`. Order in the source matches the doc's list exactly. `#[serde(rename_all = "snake_case")]` and `#[non_exhaustive]` match the pattern used by `ErrorCategory`.

---

## D.03 — Self-claim: 34 built-in patterns (Doc 4 §"Pattern Matching")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 4 at three different anchors: heading epigraph at `04-diagnosis-engine.md:3` ("Thirty-four patterns"), §"Pattern Matching" at `:170` ("The engine contains 34 built-in patterns"), and §"Why 34 Patterns" at `:312-318`. The File Reference at `:383` also says "34 patterns". Doc 14 at `14-production-failure-catalog.md:156` calls it "the 34-pattern engine".
**Reality**: Confirmed at exactly 34. `fn built_in_patterns()` at `crates/roko-conductor/src/diagnosis.rs:277-531` returns a `Vec<ErrorPattern>` with 34 literal entries (counted from the 34 `ErrorPattern {` openings in the function body at lines `281`, `288`, `295`, `302`, `309`, `316`, `323`, `330`, `337`, `345`, `352`, `360`, `368`, `375`, `383`, `390`, `398`, `405`, `413`, `420`, `428`, `436`, `443`, `450`, `458`, `465`, `472`, `479`, `486`, `493`, `501`, `508`, `515`, `523`). Other doc cross-references at `docs/07-conductor/00-conductor-architecture.md:173,362`, `docs/07-conductor/08-good-regulator-self-model.md:238,292`, `docs/07-conductor/13-process-supervision-wiring.md:453`, and `docs/07-conductor/INDEX.md:18,203` all cite "34 patterns" consistently.

Note on prompt framing: the audit prompt mentioned "27 patterns" as a possible count — that number is not correct for either the doc body or the code. The canonical figure on both sides is 34.

---

## D.04 — Doc self-regression: module doc still says "20+ error patterns"

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: The file-level doc comment at `crates/roko-conductor/src/diagnosis.rs:148` — `/// Create an engine with the built-in 20+ error patterns.` — and `:276` — `/// The default set of 20+ error patterns.` — lag the body doc's upgraded "34 patterns" claim. The test function `has_at_least_20_patterns` at `:544-546` also still uses the old lower bound.
**Reality**: Body count is 34 (D.03), all reader-facing prose in `docs/07-conductor/` says 34, but the in-crate rustdoc and the smoke test still say "20+". This is strictly consistent (34 ≥ 20) but misleading to anyone reading the crate in isolation.
**Fix sketch**: Tighten rustdoc at `diagnosis.rs:148,276` to "34 built-in error patterns" and rename the test to `has_at_least_34_patterns` or split into per-category coverage assertions so the doc claim gets enforced.

---

## D.05 — Pattern examples in Doc 4 table have drift against the code

**Status**: PARTIAL
**Severity**: LOW (doc drift, not code drift)
**Doc claim**: Doc 4 §"Pattern Examples" table at `04-diagnosis-engine.md:184-201` lists specific substring needles: `"error[E0308]"`, `"error[E0382]"`, `"error[E0106]"`, `"error[E0432]"`, `"error[E0433]"`, `"error[E0063]"`, `"cannot find"`, `"test result: FAILED"`, `"panicked at"`, `"Connection refused"`, `"rate limit"`, `"context_length_exceeded"`, `"No space left"`, `"CONFLICT"`, `"clippy::"`.
**Reality**: Most rows match exactly (E0308 at `diagnosis.rs:290`, E0382 at `:304`, E0106 at `:318`, E0432 at `:332`, "cannot find" at `:339`, "test result: FAILED" at `:347`, "Connection refused" at `:415`, "rate limit" at `:460`, "context_length_exceeded" at `:474`, "No space left on device" at `:452`). Five rows are doc-side drift:

| Doc row | Actual substring | Code ref |
|---|---|---|
| `error[E0433]` | not present (only `E0432` at `:331-336`) | — |
| `error[E0063]` | not present | — |
| `panicked at` | actual needle is `thread 'main' panicked` | `:517` |
| `CONFLICT` | actual needle is `CONFLICT (content)` | `:377` |
| `clippy::` | actual needle is generic `warning: ` | `:362` |

The code is the source of truth; the doc table is a simplified (and slightly incorrect) gloss.
**Fix sketch**: Either rewrite the Pattern Examples table to cite real needles from `built_in_patterns()`, or add `E0433` / `E0063` / `"clippy::"` / `"panicked at"` patterns to the engine so the table becomes accurate. The latter costs ~5 lines each and is probably worthwhile for `E0433` (unresolved path) because it appears in practice alongside `E0432`.

---

## D.06 — Substring `contains()` matching, not regex (Doc 4 §"Design Decisions")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 4 §"Why Substring Matching Instead of Regex" at `04-diagnosis-engine.md:295-310` claims the engine uses simple `contains()` not regex, citing performance / readability / maintainability. Doc also shows `impl DiagnosisEngine::diagnose` with `lower.contains(&p.substring.to_lowercase())` at `:209-223`.
**Reality**: Confirmed. `fn match_pattern` at `crates/roko-conductor/src/diagnosis.rs:201-225` calls `haystack.find(&needle)` (`:208`), which is substring search with `Option<usize>` return. Case sensitivity is handled by per-pattern `case_insensitive: bool` at the `ErrorPattern` level (`:110`), which doubles the `to_lowercase()` work only when that flag is set. No `regex` crate dependency in `crates/roko-conductor/Cargo.toml`. The docstring "example" at `:209-223` is slightly simplified (the real impl sorts by confidence at `:181-186` and extracts an excerpt at `:213-216`), but the substring-only claim is accurate.

---

## D.07 — Confidence scoring: ratio + specificity bonus vs doc's per-pattern static confidence

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 4 §"Pattern Examples" table at `04-diagnosis-engine.md:186-201` shows a "Confidence" column (0.70 for `"cannot find"`, 0.95 for `"error[E0308]"`, 0.90 for `"test result: FAILED"`, etc.). Doc 4 §"Pattern Matching" at `:174-181` shows an `ErrorPattern` struct with a `confidence: f64` field, implying confidence is per-pattern-static.
**Reality**: The code does not store per-pattern confidence. `pub struct ErrorPattern` at `crates/roko-conductor/src/diagnosis.rs:100-111` has no `confidence` field — only `name`, `needle`, `category`, `suggested_action`, `case_insensitive`. Confidence is computed dynamically at match time by `fn compute_confidence` at `:234-261`, which returns a value in `[0.5, 1.0]` based on (a) ratio of needle length to input length — the "base" — and (b) a specificity bonus scaled by needle length. This produces a different curve than the doc's static-per-pattern numbers; e.g., a short input containing only `"error[E0308]"` gets `ratio=1.0 → base=1.0 → confidence=1.0`, whereas the same needle inside a 10 KB compile log gets `ratio≈0.001 → base=0.5 + bonus≈0.06 → confidence≈0.56`. The doc's table-of-confidences is therefore misleading: confidences are context-dependent, not fixed.
**Fix sketch**: Either add a `base_confidence: f64` field to `ErrorPattern` and wire it through `compute_confidence` (make the static-ish doc match reality), or rewrite Doc 4's Pattern Examples table to reflect the ratio-based scheme with a single representative computation. The latter is cheaper; the former gives finer per-pattern control.

---

## D.08 — `DiagnosisEngine` wired into the CLI (call sites)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 4 §"Integration Points" at `04-diagnosis-engine.md:232-289` describes the engine being called from (a) the Conductor when watchers fire, (b) the auto-fix pipeline for `ImportError`/`E0063` classification, and (c) the learning system via `AgentEfficiencyEvent.gate_errors`.
**Reality**: Confirmed at two call sites in `orchestrate.rs`:

| Call site | File:line | Purpose |
|---|---|---|
| Circuit-breaker failure diagnosis | `crates/roko-cli/src/orchestrate.rs:3859-3869` | `DiagnosisEngine::default().diagnose(&error_output)` feeds `primary_diagnosis` + `diagnosis_results` into the `InterventionFired` event log + `conductor.circuit_breaker` signal |
| Retry-error classification | `crates/roko-cli/src/orchestrate.rs:6816-6842` | `DiagnosisEngine::default().diagnose(&chain)` maps to a `RetryErrorPattern` enum (Compile / Test / Timeout / RateLimit / ContextOverflow / Refusal / LoopDetected / Infrastructure) that governs retry strategy |

Import at `crates/roko-cli/src/orchestrate.rs:36` (`use roko_conductor::diagnosis::{DiagnosisEngine, ErrorCategory};`) matches the self-hosting-workflow call site claim. The CLAUDE.md table's "Safety layer (role auth, pre/post checks) — Wired" and "Gate pipeline — Wired" lines both rely on this diagnosis-as-classifier plumbing. Auto-fix pipeline (Doc 4 §"With the Auto-Fix Pipeline" at `:250-264`) is not explicitly wired — there's no `cargo run -p roko-cli -- autofix` subcommand, and the category-to-AutoFix mapping shown in the table at `:142-164` (where `ImportError→AutoFix`) is not exercised. The engine today sets `suggested_action: SuggestedIntervention::RetryWithContext` for `ImportError` at `diagnosis.rs:331-343`, so even the policy-table doc doesn't match the current engine output.

---

## D.09 — Category-to-Intervention table has real drift against the built-in patterns

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 4 §"Category-to-Intervention Mapping" table at `04-diagnosis-engine.md:142-164` prescribes:
- `ImportError → AutoFix`
- `TestFailure → RetryWithContext`
- `CompileError → RetryWithContext`
- `ClippyWarning → WarnAndContinue`
- `BorrowCheckerError → RestartAgent`
- `LifetimeError → RestartAgent`
**Reality**: Real `suggested_action` in `built_in_patterns()` does not match:

| Category (doc recommends) | Real action in code | Source line |
|---|---|---|
| `ImportError` → `AutoFix` | `RetryWithContext` | `diagnosis.rs:335, 342` |
| `TestFailure` → `RetryWithContext` | `AutoFix` | `:350, 357` |
| `ClippyWarning` → `WarnAndContinue` | `AutoFix` | `:365` |
| `BorrowCheckerError` → `RestartAgent` | `RetryWithContext` | `:300, 307, 314` |
| `LifetimeError` → `RestartAgent` | `RetryWithContext` | `:321, 328` |

The doc's table is aspirational / policy design; the built-in patterns all default to the most common intervention (`RetryWithContext` / `AutoFix` / `BackoffRetry` / `AbortPlan`). The high-severity inversions (`TestFailure` gets `AutoFix` in code but `RetryWithContext` in doc; `ImportError` gets `RetryWithContext` in code but `AutoFix` in doc) can meaningfully change routing cost: Doc 4 §"With the Auto-Fix Pipeline" says auto-fix costs ~$0.01 and full re-implementation ~$2.00+, so the `ImportError → AutoFix` policy is where the doc claims the $11.94 savings on 6-of-8 import errors. That savings is not being captured today.
**Fix sketch**: Reconcile at the code side — update the per-pattern `suggested_action` assignments in `built_in_patterns()` to match the doc's policy table. Add unit tests that assert `engine.diagnose("error[E0432]: ...")` yields `SuggestedIntervention::AutoFix`. Same for `error[E0063]` if D.05 is also fixed.

---

## D.10 — Self-claim: 6 `StuckKind` variants (Doc 5 §"StuckKind Enum")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 5 at `05-stuck-detection.md:41-49` enumerates exactly 6 `StuckKind` variants: `OutputLoop`, `NoProgress`, `GateLoop`, `CompileLoop`, `EmptyOutput`, `ExcessiveRetries`. Doc 5 §"Detection Heuristics" at `:76-152` describes one heuristic per variant. Cross-reference at `docs/07-conductor/INDEX.md:19` ("6 heuristics") and `docs/07-conductor/00-conductor-architecture.md:365` ("6 stuck heuristics").
**Reality**: Exact match. `pub enum StuckKind` at `crates/roko-conductor/src/stuck_detection.rs:34-47` has all 6 variants in the same order. The `StuckDetector::check_stuck` dispatcher at `:178-204` calls six `check_*` private methods (one per variant) and returns the first match in priority order. `check_all` at `:208-233` runs all six and returns every hit. The detection-heuristic doc-to-method mapping is 1-to-1:

| Doc heuristic | Code method | Default threshold |
|---|---|---|
| OutputLoop (4) | `check_output_loop` at `:278-311` | `output_loop_count: 4` at `:125` |
| NoProgress (300,000 ms) | `check_no_progress` at `:315-341` | `no_progress_ms: 300_000` at `:126` |
| GateLoop (3) | `check_gate_loop` at `:344-381` | `gate_loop_count: 3` at `:127` |
| CompileLoop (3) | `check_compile_loop` at `:384-417` | `compile_loop_count: 3` at `:128` |
| EmptyOutput (3) | `check_empty_output` at `:420-447` | `empty_output_count: 3` at `:129` |
| ExcessiveRetries (6) | `check_excessive_retries` at `:450-473` | `excessive_retry_count: 6` at `:130` |

---

## D.11 — `StuckDetector` / `MetaCognitionHook` are NOT wired into the CLI runtime

**Status**: NOT DONE
**Severity**: HIGH
**Doc claim**: Doc 5 §"MetaCognitionHook" at `05-stuck-detection.md:155-213` describes the hook as a running, theta-frequency self-assessment: "The meta-cognition check runs periodically — not on every turn (too expensive) but often enough to catch stuck agents before they burn significant budget." Doc 5 §"Signal Serialization" at `:215-234` describes the hook emitting a `Signal` with `Kind::Custom("conductor.meta_cognition")` that "feed[s] into the Conductor's signal stream, where other watchers or the intervention policy can incorporate them into the overall decision." The File Reference at `:315-319` says the hook is "Built".
**Reality**: Built, but **not wired**. `grep StuckDetector\\|MetaCognitionHook\\|check_stuck\\|meta_cognition crates/roko-cli` returns **zero matches**. Only the `roko-conductor::stuck_detection` module itself references these types (only via its own tests and docstrings). There is no `ActivityEntry` construction anywhere in `roko-cli` or `roko-orchestrator` or `roko-agent`. The `StuckDetector::check_stuck()` method is never called from any runtime path. `MetaCognitionAssessment::to_signal()` at `stuck_detection.rs:524-539` emits an `Engram` with `Kind::Custom("roko.meta_cognition")`, but no caller ever constructs the assessment. The **only** stuck-detection surface the CLI uses is the watcher-ensemble `StuckPatternWatcher` at `crates/roko-conductor/src/watchers/stuck_pattern.rs:74-122`, which is registered in `Conductor::new()` at `crates/roko-conductor/src/conductor.rs:93`. That watcher is one heuristic (`MAX_IDENTICAL_ACTIONS = 4`), overlapping with the `OutputLoop` kind only.

This leaves **5 of 6** heuristics (`NoProgress`, `GateLoop`, `CompileLoop`, `EmptyOutput`, `ExcessiveRetries`) structurally reachable but effectively dark in the running system. The 5-minute no-progress check, the gate-oscillation detector, and the `MetaCognitionAction::Escalate → Conductor Fail` escalation path described in Doc 5 §"The Self-Model Requirement" at `:238-262` are all untested in production because no code path builds the `Vec<ActivityEntry>` input.
**Fix sketch**: Wire `MetaCognitionHook::assess()` into `PlanRunner` in `orchestrate.rs` at the theta-frequency tick (end of each task turn). Build `ActivityEntry` values from the existing turn-level signals: `output_hash` from `GHOST_TURN_SIGNAL_KIND` event body, `files_changed` from `changed_files_after.len()`, `gate_result` from the per-task gate verdict, `iteration` from the executor's retry counter. Emit `assessment.to_signal()` into the conductor's signal stream so `WorstSeverityPolicy` can promote `Escalate` to `ConductorDecision::Fail`. Expected impact: catches the 5 heuristics that `StuckPatternWatcher` currently misses.

---

## D.12 — Operating-frequency doc tables match the code's `OperatingFrequency::Theta`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 5 §"Operating Frequency" at `05-stuck-detection.md:168-180` claims the hook runs at Theta ("medium, periodic") and the table at `:172-177` maps Gamma = high/every turn, Theta = medium/periodic, Delta = low/between sessions.
**Reality**: `MetaCognitionHook::frequency()` at `crates/roko-conductor/src/stuck_detection.rs:582-584` returns `OperatingFrequency::Theta` as a `const fn`. `MetaCognitionAssessment::frequency` is set to `OperatingFrequency::Theta` at construction time (`:264`). Test `meta_cognition_is_theta_frequency` at `:1040-1042` asserts this. The `OperatingFrequency` enum is owned by `roko-core` and re-exported through `stuck_detection.rs:25`. Because the hook is not actually scheduled (D.11), "runs at theta" is a static property rather than an observable runtime behavior, but the API matches the doc.

---

## D.13 — `StuckKind → MetaCognitionAction` table matches code routing logic

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 5 §"Assessment Output" at `05-stuck-detection.md:199-213` prescribes:

| Stuck Kind | MetaCognition Action |
|---|---|
| OutputLoop | AdjustStrategy |
| NoProgress | AdjustStrategy |
| GateLoop | Escalate |
| CompileLoop | Escalate |
| EmptyOutput | AdjustStrategy |
| ExcessiveRetries | AdjustStrategy |

**Reality**: Actually, the code escalates on **three** kinds, not two. `fn classify_meta_cognition_action` at `crates/roko-conductor/src/stuck_detection.rs:593-627` escalates on `GateLoop | CompileLoop | ExcessiveRetries` (`:600-606`) and adjusts strategy on `OutputLoop | EmptyOutput | NoProgress` plus `iterations_without_progress >= 3` or `repeated_output_count >= thresholds.output_loop_count` (`:608-624`). The doc's table has `ExcessiveRetries → AdjustStrategy`, but the code has `ExcessiveRetries → Escalate`. This is a LOW-severity drift because both actions would eventually restart the agent; the difference is that the code treats a 6+-iteration retry loop as "beyond single-agent retry" (Escalate = `Conductor Fail`) rather than "refresh context" (AdjustStrategy = `Conductor Restart`). Tests at `:1060-1072` (`meta_cognition_escalates_for_gate_failure_patterns`) verify the code behavior. The fact that nothing calls the hook (D.11) means this drift has no observed production impact today.
**Fix sketch**: Update Doc 5's table row `ExcessiveRetries | AdjustStrategy` → `ExcessiveRetries | Escalate` to match the code (`stuck_detection.rs:602`). Alternatively, if the doc's weaker action is intentional, move ExcessiveRetries out of the escalate branch in `classify_meta_cognition_action`.

---

## D.14 — `StuckDetector` vs watcher-ensemble overlap table (Doc 5 §"Relationship to Watcher Ensemble")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 5 §"Relationship to Watcher Ensemble" at `05-stuck-detection.md:290-311` claims the stuck detector and watcher ensemble have complementary overlapping coverage. The table at `:294-305` lists 10 detections: identical compile errors / zero output / no file changes / identical actions / gate cycling / cost overrun / context pressure / review cycling / spec drift / time overrun.
**Reality**: Watcher side is fully built — 10 watchers are registered in `Conductor::new()` at `crates/roko-conductor/src/conductor.rs:83-94` (`GhostTurn`, `ReviewLoop`, `IterationLoop`, `TestFailureBudget`, `CompileFailRepeat`, `ContextWindowPressure`, `SpecDrift`, `CostOverrun`, `TimeOverrun`, `StuckPattern`). Stuck-detector side is built but unwired (D.11). The overlaps the doc claims (CompileLoop ↔ compile-fail-repeat, EmptyOutput ↔ ghost-turn, OutputLoop ↔ stuck-pattern, GateLoop ↔ iteration-loop) are therefore one-sided — the watcher fires, the stuck heuristic does not — which means the "complementary detection" story in the doc is structurally incomplete. `NoProgress` ("not directly covered" per doc at `:298`) is real drift: no watcher covers that gap either, so the 5-minute no-file-changes signal is simply absent from the running system.
**Fix sketch**: Tied to D.11 — once `MetaCognitionHook::assess()` is called per turn and its signals feed back into the conductor's signal stream, the overlap table becomes accurate. Consider also adding a dedicated `NoProgressWatcher` to `roko-conductor/src/watchers/` so the "not directly covered" cell becomes "no-progress watcher".

---

## D.15 — Doc 14 catalog shape: 21 issues across 6 categories

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 14 §"Summary" at `14-production-failure-catalog.md:13-30` claims "21 production failures across 6 categories": State Corruption (#1-4), Data Pipeline (#5, #13-15), Process Management (#6-9), Resource Management (#10-12), Merge & Coordination (#16-18), Observability (#19-21). Cross-reference table at `:554-578` enumerates all 21.
**Reality**: All 21 entries are present (lines `38-549`), each with a Symptom / Root Cause / Conductor Response / Design Principles Violated block. Issue count is exactly 21. The cross-reference tables at `:554-605` are internally consistent — every issue appears in the Issue → Mechanism table and in the Issue → Refactoring Phase table at `:596-605` (Phase 0 through Phase 5). The catalog itself is narrative / spec material; the question is whether each "Conductor Response" claim is backed by real code.

---

## D.16 — Doc 14 State Corruption responses (#1-4) map to real circuit-breaker + watcher code

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 14 issues #1-4 (`14-production-failure-catalog.md:38-137`) cite the following conductor responses:
- #1 "in_flight/completed overlap" → Circuit breaker at `MAX_PLAN_FAILURES=2`, event-sourced state.
- #2 "Orphaned plans" → Iteration loop watcher at `MAX_ITERATIONS=3`, event-sourced state.
- #3 "Branch divergence" (ref CLAUDE.md #3/#16) → Review loop watcher at `MAX_REVIEW_CYCLES=3`, circuit breaker.
- #4 "CONTEXT.md concurrent appends" (ref CLAUDE.md #14) → Spec drift watcher at `MAX_DRIFT=0.25`, quality anomaly detector.
**Reality**: Watcher side mostly verified. `CircuitBreaker` ships at `crates/roko-conductor/src/circuit_breaker.rs` and is held by `Conductor` at `conductor.rs:59,99,119`. `IterationLoopWatcher` / `ReviewLoopWatcher` / `SpecDriftWatcher` all exist at `crates/roko-conductor/src/watchers/` and are registered in `Conductor::new()` at `conductor.rs:86,85,90`. The `handle_tripped_circuit_breaker` path in `orchestrate.rs:3844-3892` emits structured intervention signals with diagnosis. Structural prevention claims ("event-sourced state", "ephemeral branches") are design aspirations not enforced at this layer — the doc correctly labels them as `Structural prevention`, distinct from the per-watcher `Conductor Response`. The `quality anomaly detector` cited for #4 resolves to `AnomalyDetector::check_quality()` at `crates/roko-learn/src/anomaly.rs` (311 LOC), which exists but is wired from `orchestrate.rs` (`RoutingBias` + `anomaly` hits the dispatch path). Open gap: none of the four issues references the unwired `StuckDetector` / `MetaCognitionHook` from D.11, so the catalog's "Conductor Response" claims for this category are internally self-consistent but rely on the watcher side, not the stuck-detection side.

---

## D.17 — Doc 14 Data Pipeline responses (#5, #13-15) over-promise `TomlParsing` category

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 14 issue #5 at `14-production-failure-catalog.md:156-157` claims "The 34-pattern engine includes `TomlParsing` as an error category. When TOML parse failures are detected in gate output, the engine suggests `RetryWithFix` intervention." Same response text reused for issue #13 at `:176`. Issue #14 (verify script stale refs, CLAUDE.md #14) at `:195-199` claims the diagnosis engine "Matches `E0432` (unresolved import) and `E0433` (unresolved path) patterns, suggesting `ImportNotFound` category and `RetryWithFix` intervention." Issue #15 (review verdict parsing) at `:216-220` cites the review loop watcher + typed review pipeline.
**Reality**: Three real mismatches:

| Doc claim | Reality | Code ref |
|---|---|---|
| `TomlParsing` category | No such variant. `ErrorCategory` has 20 variants listed in D.01; `TomlParsing` is not one of them. Nearest match: `DependencyError` needle `"failed to select a version for"` at `:392` is Cargo-TOML-specific but doesn't catch the doc's "markdown code fences around TOML" case. | `diagnosis.rs:23-67` |
| `RetryWithFix` intervention | No such variant. `SuggestedIntervention` has 9 variants listed in D.02; closest match is `RetryWithContext` / `AutoFix`. | `diagnosis.rs:75-93` |
| `error[E0433]` pattern (Issue #14) | Not present in `built_in_patterns()`; only `E0432` at `:332`. Cross-ref D.05. | `diagnosis.rs:277-531` |
| `ImportNotFound` category (Issue #14) | No such variant. The actual category is `ImportError` at `diagnosis.rs:44`. | `diagnosis.rs:44` |

The catalog was clearly written against an earlier or aspirational version of the diagnosis API. All four rows are doc-side drift. The CompileFailRepeatWatcher and schema validation plumbing cited as secondary / structural responses are real, so issues #5 / #13-15 are partially covered — just not via the exact category/intervention names the catalog lists.
**Fix sketch**: Two options. (a) Add `TomlParsing`, `ImportNotFound` as `ErrorCategory` variants and `RetryWithFix` as a `SuggestedIntervention` variant with appropriate patterns (`"expected", "found"` for TOML parse errors from `toml::from_str`) and `E0433` pattern for unresolved paths. (b) Rewrite the Conductor Response blocks in Doc 14 issues #5 / #13 / #14 to reference the existing `DependencyError` / `ImportError` / `RetryWithContext` / `AutoFix` surface. (a) is better for reader clarity because "TOML parse" is a distinct error mode worth its own category.

---

## D.18 — Doc 14 Process Management responses (#6-9) align with real `ProcessSupervisor` + `GhostTurnWatcher`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 14 issues #6-9 at `14-production-failure-catalog.md:234-321`:
- #6 "Spawn races" → Ghost-turn watcher at `MAX_GHOST_TURNS=3`, `ProcessSupervisor` with monotonic attempt IDs.
- #7 "Orphaned cargo processes" → Cost overrun watcher, context pressure watcher, `ProcessSupervisor` with `kill_all_descendants(pid)`.
- #8 "Claude CLI cold start" → Time overrun watcher, efficiency events with `time_to_first_token_ms`.
- #9 "Agent ghost turns" (ref CLAUDE.md #9) → Ghost-turn watcher at `MAX_GHOST_TURNS=3`, stuck-pattern watcher at `MAX_STUCK_PATTERNS=4`, anomaly detector prompt-loop (5 identical prompts in 20-prompt window), cost-spike anomaly (z-score > 3.0).
**Reality**: All six named mechanisms exist and are wired. `GhostTurnWatcher` at `crates/roko-conductor/src/watchers/ghost_turn.rs:40` with `MAX_GHOST_TURNS: usize = 3` at `:11`; registered in `Conductor::new()` at `conductor.rs:84`. The CLI emits the `conductor.ghost_turn` signal from `orchestrate.rs:10775` (via `GHOST_TURN_SIGNAL_KIND` constant at `:147`). `StuckPatternWatcher` with `MAX_IDENTICAL_ACTIONS = 4` at `watchers/stuck_pattern.rs:10`. `ProcessSupervisor` ships in `roko-runtime/src/process.rs` (matches D.11's separate claim via `grep ProcessSupervisor crates/`). `AnomalyDetector::check_prompt` at `crates/roko-learn/src/anomaly.rs:52-80` matches the doc's "5 identical in 20-prompt window" exactly (`PROMPT_LOOP_WINDOW = 20, PROMPT_LOOP_THRESHOLD = 5` at `:9-10`). Cost-spike z-score 3.0 lives at `anomaly.rs:11` (`COST_SPIKE_Z_THRESHOLD: f64 = 3.0`). `CostOverrunWatcher`, `ContextWindowPressureWatcher`, `TimeOverrunWatcher` all registered at `conductor.rs:91,89,92`. This is the best-covered Doc 14 category — every mechanism the catalog names exists and is wired.

---

## D.19 — Doc 14 Resource Management responses (#10-12) have real gaps vs watcher ensemble (CLAUDE.md #12)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 14 issues #10-12 at `14-production-failure-catalog.md:326-399`:
- #10 "Disk pressure" → Health monitor (`SystemSnapshot` "can be extended"), budget anomaly detector (analogous to cost anomaly).
- #11 "Gate serialization" → Time overrun watcher, efficiency events.
- #12 "Large prompt pressure" (ref CLAUDE.md #12) → Context window pressure watcher at 80%, spec drift watcher, quality anomaly detector, structural adaptive context dropping.
**Reality**: Real gap acknowledged in the doc itself: #10 uses hedging language ("SystemSnapshot can be extended with disk pressure checks"; "budget tracking catches cost-related resource exhaustion; disk exhaustion requires an analogous disk budget"). Grep confirms: `grep disk_pressure\|DiskPressure crates/roko-conductor` returns no matches. `HealthMonitor::check_all` at `crates/roko-conductor/src/health.rs:182` runs four checks (`check_terminal_liveness`, `check_golem_status`, `check_spec_drift`, `check_coverage_trend` at lines `201,258,273,292`) — none for disk. No `DiskBudget` anywhere. #11 time overrun is wired (`TimeOverrunWatcher` at `watchers/time_overrun.rs`). #12 context window pressure + spec drift are wired (`ContextWindowPressureWatcher` + `SpecDriftWatcher`, registered at `conductor.rs:89,90`). The adaptive context dropping is via `PromptComposer` attention auction described in F.06 — partial coverage there too.
**Fix sketch**: For #10, add a `check_disk_pressure` to `HealthMonitor` against a new `SystemSnapshot::disk_free_bytes` field. Cheap: 10 LOC in `health.rs`. Consider a dedicated `DiskPressureWatcher` in `watchers/` at a threshold of 10% free (matching the March-April 2026 production incident at 7.3 GB / 1.8 TB = 0.4% free).

---

## D.20 — Doc 14 Merge & Coordination responses (#16-18) and Observability (#19-21) (CLAUDE.md #3/#16)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 14 issues #16-18 at `14-production-failure-catalog.md:408-476`:
- #16 "Rebase failures" (ref CLAUDE.md #16) → Iteration loop watcher at `MAX_ITERATIONS=3` Critical severity → Fail; circuit breaker.
- #17 "Merge conflicts at gate" → Compile-fail-repeat watcher at `MAX_COMPILE_FAILS=3`, review loop watcher, circuit breaker.
- #18 "Worktree symlinks to shared state" → Spec drift watcher, stuck-pattern watcher.
Doc 14 issues #19-21 at `:484-549`:
- #19 "Buried failures in logs" → Efficiency events, structured conductor signals.
- #20 "No signal on WHY plans fail" → **Diagnosis engine classifies into 20 categories** + intervention signals with watcher name / severity / plan ID.
- #21 "ETA completely wrong" → Anomaly detector (internal inconsistency), efficiency events with `gate_passed` as reliable progress signal.
**Reality**: Every watcher named in #16-18 is registered in `Conductor::new()` (`IterationLoopWatcher`, `CompileFailRepeatWatcher`, `ReviewLoopWatcher`, `SpecDriftWatcher`, `StuckPatternWatcher` at `conductor.rs:86,88,85,90,93`). MAX_COMPILE_FAILS / MAX_ITERATIONS / MAX_REVIEW_CYCLES all live in the respective watcher files. Ephemeral-branch "Structural prevention" claims are orthogonal to the conductor plumbing. Issues #19-21: `AgentEfficiencyEvent` is wired per CLAUDE.md line 31 ("Efficiency events (per-turn) → Wired, `.roko/learn/efficiency.jsonl` via orchestrate.rs"). The #20 claim "Diagnosis engine classifies errors into 20 categories with suggested interventions" is directly wired at `orchestrate.rs:3859-3869` (the `primary_diagnosis` payload in the `InterventionFired` event log). The #21 anomaly detector at `roko-learn/src/anomaly.rs` supplies quality-degradation detection. Remaining gap on #21: the "progress from gate outcomes, not checklist parsing" structural fix depends on pipeline plumbing beyond the conductor layer. Issue #18 stuck-pattern watcher hits only the output-repetition heuristic, not the broader stuck-detector set from D.11 — partial coverage.
**Fix sketch**: Wire D.11's `MetaCognitionHook` to close the gap for #18 (corrupted shared state → no-progress + output-loop detection). Update Doc 14 #20's "20 categories" to match D.01's actual enum, and cross-reference D.09's category-to-intervention drift so this doc doesn't re-assert a policy table that doesn't match the code.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 10 (D.01 20 ErrorCategory, D.02 9 SuggestedIntervention, D.03 34 patterns, D.06 substring match, D.08 engine wired, D.10 6 StuckKind, D.12 theta frequency, D.15 21 issues / 6 categories, D.16 state-corruption watchers wired, D.18 process-management mechanisms wired) |
| PARTIAL | 9 (D.04 rustdoc stale on 20+, D.05 pattern examples drift, D.07 confidence scoring scheme drift, D.09 category→intervention policy drift, D.13 ExcessiveRetries action drift, D.14 watcher-ensemble overlap one-sided, D.17 TomlParsing / RetryWithFix / ImportNotFound names don't exist, D.19 disk-pressure health check missing, D.20 #18/#20/#21 partial coverage) |
| NOT DONE | 1 (D.11 `StuckDetector` / `MetaCognitionHook` never called from runtime — 5 of 6 stuck heuristics effectively dark) |

The diagnosis engine surface is real and wired: 20 `ErrorCategory` variants (D.01), 9 `SuggestedIntervention` variants (D.02), 34 built-in patterns (D.03), substring-match `diagnose()` (D.06), two CLI call sites in `orchestrate.rs` at `:3859-3869` and `:6816-6842` (D.08). The patterns themselves work — the drift is in the docs, not the code.

The `StuckDetector` + `MetaCognitionHook` surface is a textbook "built but never connected" gap (see CLAUDE.md rule #2). 1,085 LOC of heuristics (`stuck_detection.rs`), full test coverage, perfectly-shaped `MetaCognitionAssessment::to_signal()` output — and **zero** runtime callers. Only `StuckPatternWatcher` (one OutputLoop-like heuristic via `Policy`) is actually firing in production. The 5-minute no-progress check described in Doc 5 §"NoProgress" is dark; the gate-oscillation detector described in Doc 5 §"GateLoop" is dark; the theta-frequency periodic self-assessment described in Doc 5 §"Operating Frequency" never runs. This is D.11 and it's the single HIGH-severity gap in this section.

The production-failure catalog (Doc 14) is mostly accurate when it points at real watchers (D.16, D.18, D.20 for State Corruption / Process Management / Observability + Merge) but asserts API names in the Data Pipeline category (#5, #13, #14) that don't exist in the code — `TomlParsing` / `RetryWithFix` / `ImportNotFound` are doc-side inventions (D.17). The Resource Management category has a real but acknowledged gap on disk-pressure health checks (D.19). CLAUDE.md's named issues (#9 ghost turns, #14 stale references, #3/#16 divergence/rebase, #12 large prompts) all map to watchers that are registered in `Conductor::new()` at `conductor.rs:83-94` and firing; the coverage story is real on the watcher side even where the diagnosis-engine names in Doc 14 drift.

Highest-leverage fix: wire `MetaCognitionHook::assess()` into the per-turn theta-frequency tick in `orchestrate.rs`, construct `ActivityEntry` values from the existing signal stream, emit `assessment.to_signal()` into the conductor stream. This turns five currently-dark heuristics (`NoProgress`, `GateLoop`, `CompileLoop`, `EmptyOutput`, `ExcessiveRetries`) into real runtime behavior and closes most of D.11, D.14, and Doc 14 #18.

Second fix: reconcile the category-to-intervention table in Doc 4 §"Category-to-Intervention Mapping" (`:142-164`) with the actual `suggested_action` assignments in `built_in_patterns()` (D.09). The doc-to-code inversions on `ImportError` (doc says `AutoFix`, code says `RetryWithContext`) and `TestFailure` (doc says `RetryWithContext`, code says `AutoFix`) are the routing-cost story the doc uses to justify $11.94 savings on 6-of-8 import errors — that savings isn't being captured today.

Third fix: add `TomlParsing` / `ImportNotFound` `ErrorCategory` variants and `RetryWithFix` `SuggestedIntervention` variant plus the relevant patterns (`E0433`, `E0063`, TOML parse errors) so Doc 14 issues #5 / #13 / #14 stop referring to APIs that don't exist.

## Agent Execution Notes

### D.11 — This Is The Main Runtime Gap In This Section

The highest-value work here is wiring `MetaCognitionHook`, not expanding the diagnosis taxonomy.

Good execution sequence:

1. make `ActivityEntry` construction possible from live runtime data,
2. call `MetaCognitionHook::assess(...)` on a real cadence,
3. prove at least one previously dark stuck heuristic can affect conductor behavior.

### D.09 / D.17 — Keep Diagnosis Cleanup Bounded

For the diagnosis/catalog drift:

- prefer making the built-in pattern table and docs agree,
- only add new enum variants if they materially improve a real production path,
- avoid turning this into a new learned diagnosis system.

Acceptance criteria for this section:

- later agents can tell which stuck heuristics are really live,
- diagnosis categories and interventions are easier to trust,
- doc 14 stops naming nonexistent APIs unless they are deliberately added.
