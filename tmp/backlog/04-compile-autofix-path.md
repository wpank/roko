# 04 — Compile Auto-Fix Path (Gap 2 completion)

**Origin**: `tmp/architecture-archive/20-orchestrator-gaps.md`, Gap 2 (lines 64–98)
**Status**: Backlog
**Priority**: P1 — directly reduces cost and latency of the gate-fix cycle
**Size**: S (1–2 days)
**Depends on**: nothing — all prerequisite types exist


## Current state (as of 2026-08-16)

Gap 2 from the architecture archive is **partially done**. The following are already
implemented and wired:

| Item | Location | State |
|---|---|---|
| `CompileError` struct | `crates/roko-gate/src/compile_errors.rs` | EXISTS |
| `ErrorCategory` enum (10 variants) | `crates/roko-gate/src/compile_errors.rs` | EXISTS |
| `classify_error_code()` | `crates/roko-gate/src/compile_errors.rs` | EXISTS |
| `GateFailureClassification.cargo_fix_candidate` | `crates/roko-gate/src/compile_errors.rs` | EXISTS |
| `attempt_auto_fix()` | `crates/roko-cli/src/runner/gate_dispatch.rs:391` | EXISTS |
| `AutoFixOutcome` | `crates/roko-cli/src/runner/gate_dispatch.rs:356` | EXISTS |
| Auto-fix wired into `run_gate_once()` | `crates/roko-cli/src/runner/gate_dispatch.rs:539–596` | EXISTS |
| `GatesConfig.cargo_fix_enabled` (default `true`) | `crates/roko-core/src/config/gates.rs:58` | EXISTS |

What remains open from Gap 2:

1. **`collect_rustc_suggestions()` as a named, tested function** — the JSON extraction
   logic for `children[].suggested_replacement` spans exists in `parse_cargo_json()` but
   is not exposed as a standalone callable. The architecture spec names this function
   explicitly as a contract for other subsystems (e.g., reflection loop, pattern learning)
   that may want to inspect available fixes without triggering the full auto-fix flow.

2. **Classified errors in agent prompt** — `build_gate_retry_context()` in `event_loop.rs`
   already calls `classify_gate_failure()` and includes `render_failure_classification()`
   output in the retry prompt. However the structured `compile_errors` list from
   `GateFailureClassification` is not formatted separately as a readable per-error table
   in the prompt — only the raw JSON classification blob is included. Agents receive
   the classification but must parse JSON themselves to extract file/line/suggestion detail.


## Problem statement

Every compile gate failure currently takes the same path: gate fails, agent gets raw
cargo output + JSON classification blob, agent retries with a full LLM turn. When
`rustc` itself knows the fix and has emitted a machine-applicable suggestion, this
full agent turn is wasted: `cargo fix --allow-dirty` would have resolved the error in
under a second.

The gate-then-agent cycle costs roughly $0.50–2.00 per fix attempt (depending on model,
context window, and task complexity). For purely mechanical errors — unused imports,
simple type annotation fixes, trivial lint violations — this cost is entirely avoidable.

Additionally, when the auto-fix attempt fails or the gate failure requires an agent,
the agent currently sees a raw JSON blob for the structured classification rather than
a human-readable per-error table with file, line, category, and suggestion text. This
makes it harder for the agent to pinpoint which files and lines to fix.


## What exists vs what needs building

### Already implemented (do NOT rebuild)

```rust
// crates/roko-gate/src/compile_errors.rs

// Detects whether any error has a machine-applicable suggestion
pub struct GateFailureClassification {
    pub cargo_fix_candidate: bool,   // true = at least one error has a suggestion
    pub compile_errors: Vec<CompileError>,
    // ...
}

// Parses --message-format=json output into structured errors
pub fn parse_cargo_json(stderr: &str) -> CompileErrorSummary

// Full classification pipeline
pub fn classify_gate_failure(gate: &str, output: &str) -> GateFailureClassification
```

```rust
// crates/roko-cli/src/runner/gate_dispatch.rs

// Runs cargo fix --allow-dirty (compile gates) or cargo clippy --fix --allow-dirty
// (clippy gates), then cargo fmt. Returns outcome struct.
pub async fn attempt_auto_fix(
    workdir: &Path,
    gate_name: &str,
    error_output: &str,
) -> Result<AutoFixOutcome, String>

// Wired into run_gate_once() at lines 539-596:
// gate fails → attempt_auto_fix() → re-gate → if pass, use retry verdicts
```

```rust
// crates/roko-core/src/config/gates.rs

pub struct GatesConfig {
    pub cargo_fix_enabled: bool,  // default: true; disables auto-fix path when false
    // ...
}
```

### Still missing

**1. `collect_rustc_suggestions()` as a public, tested function**

The extraction logic exists inside `parse_cargo_json()` but is not exposed as a named
public function. Gap 2's original spec (architecture-archive line 84) names this
function explicitly as a first-class API because it is narrower in scope than
`parse_cargo_json()` — it returns only the suggested replacements, without the full
error classification machinery, for use by callers that want to know whether
compiler-guided fixes are available before deciding to run `cargo fix`.

Location: `crates/roko-gate/src/compile_errors.rs`

```rust
/// Extract rustc-suggested replacements from cargo JSON diagnostic output.
///
/// Scans each `compiler-message` for `children` entries whose `level` is
/// `"help"` or `"suggestion"`. Returns the message text of each such child.
/// Unlike `parse_cargo_json`, this never constructs `CompileError` structs
/// and never touches error/warning counts — it is a pure suggestion extractor.
///
/// Returns an empty `Vec` when no suggestions are present (never `Err`).
pub fn collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion>

pub struct RustcSuggestion {
    /// The suggested fix text (from `children[].message`).
    pub text: String,
    /// The file the suggestion applies to, if the span is available.
    pub file: Option<String>,
    /// Starting line of the span, if available.
    pub line: Option<u32>,
}
```

**2. Human-readable classified error table in the agent retry prompt**

`build_gate_retry_context()` in `event_loop.rs` (line 15315) includes the full JSON
classification blob. The gap is that the structured `compile_errors` list is not
rendered as a readable per-error table in the prompt. Agents can extract this
information from JSON but benefit from having it pre-formatted.

The fix is to add a `format_compile_errors_for_prompt()` helper and call it inside
`build_gate_retry_context()` when `classification.compile_errors` is non-empty, inserting
a `### Structured compile errors` section before the raw gate output block.

Location: `crates/roko-cli/src/runner/event_loop.rs`

```rust
fn format_compile_errors_for_prompt(errors: &[CompileError]) -> String {
    // Renders each error as:
    //   [E0308 / TypeMismatch] src/lib.rs:42 — expected `u32`, found `String`
    //     Suggestion: change `x` to `x as u32`
    // Cap at 20 errors. If more exist, append "... and N more errors".
}
```


## Full intended flow

```
compile gate fails
        |
        v
classify_gate_failure(gate_name, output)
        |
        +-- cargo_fix_candidate == false --> skip to "pass classified errors to agent"
        |
        +-- cargo_fix_candidate == true
                |
                v
        attempt_auto_fix(workdir, gate_name, output)   [already wired in run_gate_once]
                |
                +-- fix exited non-zero --> skip (fall through to agent)
                |
                +-- fix applied
                        |
                        v
                re-run gate pipeline
                        |
                        +-- pass --> use retry verdicts, skip agent entirely
                        |            (saves $0.50-2.00 per cycle)
                        |
                        +-- fail --> pass classified errors to agent
                                    (current path, needs prompt improvement per item 2)
```

The flow for item 1 (`collect_rustc_suggestions`) sits at the classification step —
callers that want to present a "N suggestions available" summary in the TUI or
dashboard can call it without invoking the full fix machinery.


## Implementation plan

### Step 1: `collect_rustc_suggestions()` in roko-gate

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/compile_errors.rs`

Add `RustcSuggestion` struct and `collect_rustc_suggestions()` function. The logic
reuses the JSON scanning from `parse_cargo_json()` but only extracts `children` entries
with `level == "help" || level == "suggestion"`, recording message text, file name
from the child's first span (if present), and line number.

Add unit tests covering:
- JSON input with one suggestion → `Vec` of length 1 with correct text
- JSON input with no suggestions → empty `Vec`
- Malformed JSON lines → silently skipped, no panic
- Multiple diagnostics → all suggestions collected

### Step 2: Human-readable classified error table in retry prompt

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

Add `format_compile_errors_for_prompt(errors: &[CompileError]) -> String` (private).
Update `build_gate_retry_context()` to call it and insert the section when
`classification.compile_errors` is non-empty.

Format per error:
```
[E0308 / type_mismatch] src/lib.rs:42 — expected `u32`, found `String`
  Suggestion: change `x` to `x as u32`
```

Cap at 20 errors with `... and N more errors` suffix. This keeps the prompt manageable
even for large compile failure batches.

Add unit tests covering:
- Compile error output with a known `E0308` → prompt contains `### Structured compile errors`
- Zero compile errors → section absent from prompt
- 25 errors → prompt shows 20 with truncation note


## Key constraints

- **Never use `--allow-staged`** in any `cargo fix` invocation. The spec (architecture-archive
  lines 505–512) is explicit: `--allow-staged` can corrupt the staging area. Only
  `--allow-dirty` is permitted.
- **Non-zero exit from `cargo fix` → fall through to agent**, never abort the runner.
  This is already implemented in `attempt_auto_fix()` at gate_dispatch.rs:425–437.
- **`cargo_fix_enabled = false` must skip the auto-fix path entirely.** Already
  enforced at gate_dispatch.rs:544. The config toggle must be respected.
- **Gate input immutability check surrounds the fix.** `run_gate_once()` takes a
  `before` snapshot, runs the fix and retry, takes an `after` snapshot. If
  `before != after` the retry verdicts are discarded. This is already implemented.
- The auto-fix path only applies to gates whose raw name starts with `"compile"` or
  `"clippy"`. Test gates, docs gates, and custom shell gates are never passed to
  `attempt_auto_fix()` for fix purposes (though they may still be classified).


## Acceptance criteria

1. `collect_rustc_suggestions(json_output)` is a public function in `roko-gate` that
   returns `Vec<RustcSuggestion>` and has unit tests covering the empty, single, and
   multi-suggestion cases. It does not duplicate `parse_cargo_json()` logic — it
   delegates to the same JSON scanning path but extracts only suggestion text.

2. `build_gate_retry_context()` in `event_loop.rs` includes a `### Structured compile errors`
   section when `classification.compile_errors` is non-empty, with each error rendered
   as `[code / category] file:line — message` plus suggestion text when present. The
   section is absent (not just empty) when there are no compile errors.

3. When `cargo fix --allow-dirty` succeeds and the retry gate passes, the runner
   proceeds to the next task without spawning an agent. The `AutoFixOutcome` is logged
   with `gate_passed_after_fix = true`. This path is already wired; the acceptance
   criterion is that it is covered by an integration test that provides a real cargo
   workspace with a fixable error and asserts the agent is not dispatched.

4. When `cargo fix` exits non-zero (conflicting suggestions, macro-generated code,
   etc.), the runner falls through to the agent with no state corruption. The working
   tree must be in the same state as before the fix attempt. This is already enforced;
   the acceptance criterion is a unit test that mocks a non-zero exit and asserts
   `AutoFixOutcome { fix_applied: false, gate_passed_after_fix: false }`.

5. `GatesConfig.cargo_fix_enabled = false` in `roko.toml` disables the auto-fix path.
   When disabled, `run_gate_once()` skips the `attempt_auto_fix()` branch entirely and
   the agent receives the gate failure immediately. Covered by the existing TOML
   round-trip test at gate_dispatch.rs:2123.

6. `cargo clippy --fix --allow-dirty` is used for clippy gate failures (not
   `cargo fix`). This is already implemented in `attempt_auto_fix()` at gate_dispatch.rs:405;
   the acceptance criterion is a unit test that asserts the command string contains
   `clippy --fix` when the gate name starts with `"clippy"`.


## Cost and latency impact

A typical fixable compile error (unused import, trivial type annotation, lint violation)
resolved by `cargo fix` takes approximately 0.5–3 seconds on a warm build cache.
An agent retry for the same error costs:

- Model API latency: 8–30 seconds
- Token cost: $0.20–2.00 depending on context window size and model tier
- Total wall-clock cost per fix cycle: 30–90 seconds

The auto-fix path intercepts these cycles before the agent is dispatched, so any run
where `cargo fix` resolves the failure eliminates the full agent cost. In practice,
approximately 20–40% of compile failures during active development are in categories
that rustc can suggest fixes for (unused imports, simple lifetime annotations, obvious
type coercions). At $0.50–2.00 per cycle, even modest reduction in fix-cycle count
produces measurable savings across a multi-task plan.


## References

- `tmp/architecture-archive/20-orchestrator-gaps.md` lines 64–98 (Gap 2 spec)
- `tmp/architecture-archive/20-orchestrator-gaps.md` lines 504–512 (Gap 2 spec clarification: merge conflict handling)
- `crates/roko-gate/src/compile_errors.rs` — `CompileError`, `ErrorCategory`, `classify_error_code()`, `GateFailureClassification`, `cargo_fix_candidate`
- `crates/roko-cli/src/runner/gate_dispatch.rs:354–457` — `AutoFixOutcome`, `attempt_auto_fix()`
- `crates/roko-cli/src/runner/gate_dispatch.rs:539–596` — auto-fix wired into `run_gate_once()`
- `crates/roko-cli/src/runner/event_loop.rs:15315–15351` — `build_gate_retry_context()`
- `crates/roko-core/src/config/gates.rs:58` — `GatesConfig.cargo_fix_enabled`
