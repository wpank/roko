# D — Feedback & Thresholds (Docs 06, 08)

Parity analysis of `docs/04-verification/06-adaptive-thresholds.md` and
`docs/04-verification/08-agent-feedback-from-gates.md` vs the actual codebase.

---

## D.01 — `AdaptiveThresholds` per-rung EMA tracker

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §2, §3 — `RungStats { ema_pass_rate, total_observations, consecutive_passes }` keyed per rung; neutral prior 0.5; EMA α = 0.1.
**Reality**: `crates/roko-gate/src/adaptive_threshold.rs:11-47`:
- `EMA_ALPHA = 0.1` at line 11
- `MIN_RETRIES = 1`, `MAX_RETRIES = 5` at lines 14-16
- `SKIP_STREAK_THRESHOLD = 20` at line 19
- `RungStats::default()` starts `ema_pass_rate = 0.5` (line 35)

Exact match with doc.

---

## D.02 — EMA update rule

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §3.1 — first observation sets rate directly; otherwise `new = α·value + (1-α)·old`.
**Reality**: `adaptive_threshold.rs:79-96`:
```rust
if stats.total_observations == 0 {
    stats.ema_pass_rate = value;
} else {
    stats.ema_pass_rate = EMA_ALPHA.mul_add(value, (1.0 - EMA_ALPHA) * stats.ema_pass_rate);
}
```
Byte-identical semantics to doc's algorithm.

---

## D.03 — `suggested_max_retries(rung)` advisory

**Status**: PARTIAL (HIGH severity)
**Doc claim**: Doc 06 §4, §7.2 — orchestrator calls `suggested_max_retries` before its retry loop; pass rate 1.0 → 1 retry, 0.5 → 3, 0.0 → 5.
**Reality**: Mapping logic is correct — `adaptive_threshold.rs:102-119` implements linear pass-rate-to-retries mapping with 5-observation cold-start returning 3. **But**: the **orchestrator does not consume this**. Call sites of `suggested_max_retries` grep to:
- `crates/roko-cli/src/main.rs:5439` — CLI status/report command only
- `crates/roko-serve/src/routes/learning.rs:237` — HTTP API exposure only
- `crates/roko-cli/src/tui/dashboard.rs:2708, 3991` — dashboard display only
- Tests in `adaptive_threshold.rs` (168, 178)

**Not called** from `orchestrate.rs` anywhere. AutoFix retry path at `orchestrate.rs:8897` uses a fixed retry budget, ignoring the advisory.
**Fix sketch**: Before the AutoFix retry loop in `handle_autofix`, call `self.adaptive_thresholds.suggested_max_retries(current_rung)` and cap the loop to that value. This closes the half-open feedback loop.

---

## D.04 — `should_skip_rung(rung)` advisory

**Status**: PARTIAL (HIGH severity)
**Doc claim**: Doc 06 §5, §7.1 — when `consecutive_passes >= 20`, skip advisory fires. Rung selector should intersect this with the static selection.
**Reality**: Implementation at `adaptive_threshold.rs:127-131` is correct. Callers:
- `crates/roko-cli/src/main.rs:5441` — CLI report
- `crates/roko-serve/src/routes/learning.rs:238` — HTTP
- `crates/roko-cli/src/tui/dashboard.rs:3687, 4015` — dashboard

**Not called** from `orchestrate.rs`. The hardcoded `run_gate_rung` (B.04) never consults `should_skip_rung` before dispatching. Skipping does not happen at runtime.
**Fix sketch**: When B.04 is fixed and `run_gate_rung` is registry-driven, filter the rung vec through `should_skip_rung` before building the `GatePipeline`.

---

## D.05 — Update-half of the feedback loop (runtime → EMA)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §7.3 — after each pipeline execution, `thresholds.update(rung, passed)` then `thresholds.save(path)`.
**Reality**: `orchestrate.rs:5329` calls `self.adaptive_thresholds.update(rung, passed)` after gate runs; `orchestrate.rs:3741` calls `self.adaptive_thresholds.save(&thresholds_path)`. Load paths at `orchestrate.rs:3292, 3411, 3534` all call `AdaptiveThresholds::load_or_new(...gate-thresholds.json)`. The feedback loop is **half closed**: gate outcome → EMA update → persist. What's missing is the half that *acts* on the stored EMAs (D.03, D.04).

---

## D.06 — Persistence: atomic write to `.roko/learn/gate-thresholds.json`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 §6 — atomic write via tmp-then-rename; graceful degradation via `load_or_new` returning fresh on corrupt file.
**Reality**: `adaptive_threshold.rs:66-76` — `save` writes to `.json.tmp` then `rename`; creates parent directory with `create_dir_all`. `load_or_new` at `adaptive_threshold.rs:58-63` swallows IO and JSON errors with `.ok().and_then(...).unwrap_or_default()`. Test `round_trip_persistence` at `adaptive_threshold.rs:200-213` confirms roundtrip.

---

## D.07 — SPC extensions (doc 06 §11–16): CUSUM, EWMA control chart, BOCPD, PELT, Hotelling T²

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 06 §11 introduces `CusumDetector`; §11.2 `EwmaControlChart`; §11.3 `BocpdDetector`; §12 `MultiGateDetector` using Hotelling's T²; §13 domain profiles; §14 `PeltDetector`; §15 `RungStatsExtended` with all SPC detectors.
**Reality**: Grep `grep -rn 'CusumDetector\|EwmaControlChart\|BocpdDetector\|PeltDetector\|MultiGateDetector' crates/` returns **zero matches**. No ShiftDirection enum, no NormalGammaStats, no RungHealth composite type, no ThresholdProfile / CusumParams. Doc §13's `profile_for_role` (code-writer/test-writer/refactoring profiles) is absent.
**Fix sketch**: Doc 06 §11-§16 is ~700 lines of detector design that does not exist in code. Explicit labeling "design — not started, the current implementation is §1–§10 only" would prevent reader confusion. SPC extensions are not blocking self-hosting.

---

## D.08 — `GateFeedback` struct + three-severity classifier

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §2 — `GateFeedback { rung, passed, errors, warnings, suggestions }` with `item_count`, `is_empty`, `items()` ordering errors first.
**Reality**: `crates/roko-gate/src/feedback.rs:47-94`:
- Struct at lines 52-64 has exactly the doc's fields.
- `item_count()` lines 67-71, `is_empty()` lines 74-77, `items()` lines 80-94 (errors → warnings → suggestions).
- `Severity` enum at lines 13-21: `Info < Warning < Error` ordering via `PartialOrd/Ord`.

Re-exported from `lib.rs:50`.

---

## D.09 — Line classifier pipeline (noise / error / warning / suggestion)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §3.1-3.5 — per-line priority chain: empty → noise → error → warning → suggestion → drop.
**Reality**: `feedback.rs:99-131` implements exactly this order. Noise detectors (§3.2) at `feedback.rs:134-160` cover `Downloading/Downloaded/Compiling/Checking/Finished/Running/Documenting/Fresh/Packaging` plus `npm WARN ... deprecated` plus Unicode progress bars (`━/▓/░`). Error patterns (§3.3) at `feedback.rs:163-172` cover `error/Error:/ERROR:/FAILED/FAIL /error[E/panicked at/thread '...' panicked`. Warning patterns (§3.4) at lines 175-180. Suggestion patterns (§3.5) at lines 183-192 (help/note/suggestion/hint/`-->`). Every pattern claimed in doc is present in code.

---

## D.10 — `feedback_for_agent(gate_output, rung)` public API

**Status**: PARTIAL (HIGH severity)
**Doc claim**: Doc 08 §4 — entry point that takes raw gate output and rung, returns structured `GateFeedback`. Doc §5 describes the retry-prompt injection pattern. Doc §6 claims 97.75% token reduction.
**Reality**: The function exists at `feedback.rs:196-237` and works correctly — 14 unit tests at `feedback.rs:241-374` confirm behavior. **But**: grep `grep -rn 'feedback_for_agent\|GateFeedback' crates/` returns matches **only in**:
- `crates/roko-gate/src/lib.rs:50` (re-export)
- `crates/roko-gate/src/feedback.rs` (impl + tests)

Zero callers in orchestrate.rs. The AutoFix retry path at `orchestrate.rs:8897-9000` uses raw verdict `detail` text via `format_gate_failure_context` — agents receive unfiltered stdout/stderr, not the classified feedback structure. The 97.75% token reduction claim is theoretical — the module computes it but nothing consumes the result.
**Fix sketch**: In `handle_autofix`, replace the raw-detail injection with `feedback_for_agent(&verdict.detail, rung)` and format errors/warnings/suggestions into the prompt using the doc §5 pattern. This is the most impactful token-saving wiring opportunity.

---

## D.11 — Serde roundtrip for `GateFeedback`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §9 — both `GateFeedback` and `FeedbackItem` derive `Serialize + Deserialize`.
**Reality**: `feedback.rs:26` and `feedback.rs:52` both derive serde. Test `feedback_serde_roundtrip` at `feedback.rs:367-373` confirms. Enables feedback to be persisted to episode logs (though that persistence is not currently wired either).

---

## D.12 — Multi-line error message grouping (doc 08 §10.2 limitation)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 08 §10.2 acknowledges as a known limitation: each line is classified independently, so rustc multi-line diagnostics are split (error line → Error, source snippet → dropped, help → Info).
**Reality**: Doc's own limitation is accurate — `classify_line` processes one line at a time with no look-ahead/back context. No grouping logic exists. Source snippets matching `-->` do get captured as Info (confirmed by `feedback_mixed_output` test), but non-`-->` source lines between an error and its help are dropped.
**Fix sketch**: Phase 2 improvement — track "current open diagnostic" across lines and group until blank-line terminator. Low priority since current behavior already gives agents the error + location.

---

## D.13 — Structured output parsing (doc 08 §10.3)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 08 §10.3 sketches auto-detecting JSON/SARIF output and parsing it instead of line-by-line.
**Reality**: No structured-output detection code. `feedback_for_agent` always treats input as line-based text.
**Fix sketch**: Not currently blocking — rustc plain output is the dominant format in a Rust codebase. Revisit if integrating with tools emitting SARIF.

---

## D.14 — Per-build-system classifier (doc 08 §10.1)

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 08 §10.1 — future work, dispatch classifier by `BuildSystem` (Cargo/Npm/Go/Make).
**Reality**: Current classifier is Rust/Cargo-biased. `npm WARN deprecated` is handled explicitly; Go test `FAIL ` is handled explicitly; but there's no per-BuildSystem dispatch. Fine for a Rust-first codebase.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 |
| PARTIAL | 3 (D.03, D.04, D.10 — all "built, not wired") |
| NOT DONE | 4 (D.07 SPC extensions, D.12 grouping, D.13 structured, D.14 per-BS classifier) |

Two themes here.

**Theme 1: Half-open advisory loop**. `AdaptiveThresholds` has the update-and-persist side wired correctly (D.05, D.06) but the read-and-act side bypasses orchestrate and only surfaces in CLI/dashboard/HTTP (D.03, D.04). The EMAs train themselves on real data but never affect runtime retry budgets or rung skipping.

**Theme 2: `GateFeedback` is unused from the agent retry path**. 374 LOC of classifier + 14 tests, but zero callers in orchestrate.rs. Wiring `feedback_for_agent` into `handle_autofix` is one of the highest-value, lowest-effort changes available — it's the token-economy claim from doc §6, currently only theoretical.

SPC extensions (D.07) are a substantial design-only section (700 lines of doc, zero code) and should be explicitly marked as such so readers don't misread doc 06 as spec-for-current-code.

## Agent Execution Notes

### D.03 / D.04 — Adaptive Threshold Read-Side

This is one of the main runtime-activation batches in `04`.

Recommended slice:

1. use `suggested_max_retries` on one real retry path,
2. use `should_skip_rung` when building or filtering live gate runs,
3. log or test the resulting decisions clearly.

Acceptance criteria:

- adaptive thresholds affect runtime,
- cold-start behavior stays explicit,
- the patch does not stop at dashboard-only visibility.

### D.08-D.10 — Structured AutoFix Feedback

This is a good narrow overnight batch.

Recommended slice:

1. route gate output through `feedback_for_agent`,
2. preserve `errors`, `warnings`, and `suggestions` distinctly in the prompt,
3. keep raw detail available elsewhere if needed for debugging.

Acceptance criteria:

- AutoFix sees structured verification feedback,
- token-shaping is real on a production path,
- the patch does not pretend multi-line grouping or SARIF support already exists.

### D.07 / D.12-D.14 — Defer By Default

Do not default to implementing SPC detectors, multi-line diagnostic grouping, structured parsers, or per-build-system classifiers in batch `04` unless a narrower runtime task proves they are required.
