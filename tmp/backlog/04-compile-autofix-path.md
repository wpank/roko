# 04 — Compile Auto-Fix Path (Gap 2 completion)

**Priority**: P1 — Eliminates $0.50-2.00 LLM agent turns for mechanically fixable compile errors
**Size**: S (1-2 days)
**Crates**: `crates/roko-gate` (new function), `crates/roko-cli` (runner prompt improvement)
**Depends on**: None — all prerequisite types already exist

---

## Background

Roko runs agents that write Rust code. After each agent turn, the runner executes a gate pipeline: compile (`cargo build --workspace`), lint (`cargo clippy`), and test (`cargo test`). When a gate fails, the failure is handed back to the agent as context for a retry turn.

Each agent retry turn costs money and time: 8-30 seconds of API latency, and $0.20-2.00 in token cost depending on context size and model tier. For a typical plan with several compile failures during active development, these costs accumulate rapidly.

However, a significant fraction of compile failures (roughly 20-40%) are *mechanically fixable* — rustc itself knows the fix and emits a machine-applicable suggestion in its JSON diagnostic output. For these cases, `cargo fix --allow-dirty` can resolve the failure in under 3 seconds without any LLM call. Roko already has this auto-fix path implemented and wired into the gate runner.

The problem is that two specific pieces of the auto-fix infrastructure are incomplete:

1. **`collect_rustc_suggestions()` is not a standalone public function.** The suggestion extraction logic exists inside `parse_cargo_json()` in `crates/roko-gate/src/compile_errors.rs`, but it is not exposed as a named, callable, tested function. Other subsystems (TUI, dashboard, post-gate reflection) cannot check whether compiler suggestions are available without invoking the full parse pipeline.

2. **The agent retry prompt shows a raw JSON blob, not a human-readable error table.** When `cargo fix` fails or is not applicable, the agent retry prompt (built in `build_gate_retry_context()` at `crates/roko-cli/src/runner/event_loop.rs` line 15323) includes the full JSON classification blob via `render_failure_classification()`. The agent must parse JSON to extract file, line, error code, and suggestion. A pre-formatted table would be significantly easier for the agent to act on.

## Current State

The following are **already implemented and wired** — do not rebuild:

1. **`CompileError` struct** at `crates/roko-gate/src/compile_errors.rs` line 160 — fields: `category`, `code`, `message`, `file`, `line`, `column`, `suggestion`.

2. **`ErrorCategory` enum** at line 9 — 11 variants: `Syntax`, `UnresolvedImport`, `TypeMismatch`, `Lifetime`, `MissingMember`, `Unused`, `Visibility`, `Macro`, `TraitBound`, `Ownership`, `Other`.

3. **`classify_error_code()`** at line 252 — maps rustc error codes to `ErrorCategory`.

4. **`parse_cargo_json()`** at line 324 — parses `--message-format=json` output into `CompileErrorSummary`. Already extracts `children[].message` for `level == "help"` or `"suggestion"` into `CompileError.suggestion`.

5. **`GateFailureClassification.cargo_fix_candidate`** at line 203 — `true` when any `CompileError` has a non-empty suggestion.

6. **`classify_gate_failure()`** at line 491 — full classification pipeline.

7. **`attempt_auto_fix()`** at `crates/roko-cli/src/runner/gate_dispatch.rs` line 391 — runs `cargo fix --allow-dirty` (compile gates) or `cargo clippy --fix --allow-dirty` (clippy gates), then `cargo fmt`. Returns `AutoFixOutcome`.

8. **`AutoFixOutcome`** at gate_dispatch.rs line 354 — fields: `gate_name`, `was_candidate`, `fix_applied`, `gate_passed_after_fix`, `command`.

9. **Auto-fix wired into `run_gate_once()`** at gate_dispatch.rs lines 539-596 — gate fails → `attempt_auto_fix()` → re-gate → if pass, use retry verdicts without agent.

10. **`GatesConfig.cargo_fix_enabled`** at `crates/roko-core/src/config/gates.rs` line 58 — defaults to `true`; disables auto-fix when `false`.

11. **TOML round-trip test** for `cargo_fix_enabled` at gate_dispatch.rs line 2123-2136.

**What is missing:**

- **`collect_rustc_suggestions()` as a public, standalone function** in `crates/roko-gate/src/compile_errors.rs`.
- **`format_compile_errors_for_prompt()` helper** and its use in `build_gate_retry_context()` in `crates/roko-cli/src/runner/event_loop.rs`.

## Implementation Plan

### Step 1: Add `collect_rustc_suggestions()` to `crates/roko-gate/src/compile_errors.rs`

Add a `RustcSuggestion` struct and the `collect_rustc_suggestions()` function after the existing `parse_cargo_json()` function (after line 425):

```rust
/// A rustc-suggested fix extracted from cargo JSON diagnostic output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RustcSuggestion {
    /// The suggestion message text (from `children[].message`).
    pub text: String,
    /// File the suggestion applies to, if the primary span is available.
    pub file: Option<String>,
    /// Starting line of the suggestion span, if available.
    pub line: Option<u32>,
}

/// Extract rustc-suggested replacements from cargo JSON diagnostic output.
///
/// Scans each `compiler-message` for `children` entries whose `level` is
/// `"help"` or `"suggestion"`. Returns the message text of each such child.
///
/// Unlike `parse_cargo_json`, this function never constructs `CompileError`
/// structs and never touches error/warning counts — it is a pure suggestion
/// extractor. Callers that only need to know whether suggestions are available
/// (e.g. TUI, dashboard) can call this without the full classification machinery.
///
/// Returns an empty `Vec` when no suggestions are present. Never returns `Err`.
pub fn collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion> {
    let mut suggestions = Vec::new();

    for line in json_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Silently skip malformed JSON lines.
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        if msg.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }

        let Some(message) = msg.get("message") else {
            continue;
        };

        // Extract suggestion from children with level == "help" or "suggestion".
        let children = message
            .get("children")
            .and_then(|c| c.as_array());

        let Some(children) = children else { continue };

        for child in children {
            let level = child.get("level").and_then(|l| l.as_str()).unwrap_or("");
            if level != "help" && level != "suggestion" {
                continue;
            }
            let text = child
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                continue;
            }

            // Extract file/line from the child's first span, if present.
            let (file, line_num) = child
                .get("spans")
                .and_then(|s| s.as_array())
                .and_then(|spans| spans.first())
                .map(|span| {
                    let file = span
                        .get("file_name")
                        .and_then(|f| f.as_str())
                        .map(String::from);
                    let line = span
                        .get("line_start")
                        .and_then(|l| l.as_u64())
                        .and_then(|l| u32::try_from(l).ok());
                    (file, line)
                })
                .unwrap_or((None, None));

            suggestions.push(RustcSuggestion { text, file, line: line_num });
        }
    }

    suggestions
}
```

Add unit tests immediately below the new function (or in the existing `#[cfg(test)]` block at the bottom of the file):

```rust
#[test]
fn collect_rustc_suggestions_single_help_child() {
    let json_line = r#"{"reason":"compiler-message","message":{"message":"cannot find value `foo`","code":{"code":"E0425","explanation":null},"level":"error","spans":[{"file_name":"src/main.rs","byte_start":0,"byte_end":3,"line_start":1,"line_end":1,"column_start":1,"column_end":4,"is_primary":true}],"children":[{"message":"consider importing this","level":"help","spans":[{"file_name":"src/main.rs","line_start":1,"column_start":1,"is_primary":true}]}]}}"#;
    let suggestions = collect_rustc_suggestions(json_line);
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].text, "consider importing this");
    assert_eq!(suggestions[0].file.as_deref(), Some("src/main.rs"));
}

#[test]
fn collect_rustc_suggestions_empty_when_no_children() {
    let json_line = r#"{"reason":"compiler-message","message":{"message":"type mismatch","code":{"code":"E0308"},"level":"error","spans":[],"children":[]}}"#;
    let suggestions = collect_rustc_suggestions(json_line);
    assert!(suggestions.is_empty());
}

#[test]
fn collect_rustc_suggestions_skips_malformed_lines() {
    let input = "not json at all\n{\"reason\":\"other\"}\n";
    let suggestions = collect_rustc_suggestions(input);
    assert!(suggestions.is_empty()); // no panic, just empty
}

#[test]
fn collect_rustc_suggestions_multiple_diagnostics() {
    // Two compiler-message entries, each with one help child.
    let line1 = r#"{"reason":"compiler-message","message":{"message":"err1","code":null,"level":"error","spans":[],"children":[{"message":"fix 1","level":"help","spans":[]}]}}"#;
    let line2 = r#"{"reason":"compiler-message","message":{"message":"err2","code":null,"level":"error","spans":[],"children":[{"message":"fix 2","level":"suggestion","spans":[]}]}}"#;
    let input = format!("{line1}\n{line2}\n");
    let suggestions = collect_rustc_suggestions(&input);
    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0].text, "fix 1");
    assert_eq!(suggestions[1].text, "fix 2");
}
```

### Step 2: Add `format_compile_errors_for_prompt()` to `crates/roko-cli/src/runner/event_loop.rs`

Add a private helper function before `build_gate_retry_context()` (before line 15323):

```rust
/// Format a list of structured compile errors as a human-readable table
/// for inclusion in agent retry prompts.
///
/// Each error is rendered as:
///   `[E0308 / type_mismatch] src/lib.rs:42 — expected `u32`, found `String``
///   `  Suggestion: change `x` to `x as u32``
///
/// Capped at 20 errors. If more exist, appends `"... and N more errors"`.
fn format_compile_errors_for_prompt(errors: &[roko_gate::CompileError]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let mut out = String::from("### Structured compile errors\n\n");
    let display_count = errors.len().min(20);

    for error in &errors[..display_count] {
        let code_str = error.code.as_deref().unwrap_or("?");
        let category_str = format!("{:?}", error.category).to_lowercase();
        let location = match (error.file.as_deref(), error.line) {
            (Some(f), Some(l)) => format!("{f}:{l}"),
            (Some(f), None) => f.to_string(),
            _ => "unknown location".to_string(),
        };
        out.push_str(&format!(
            "[{code_str} / {category_str}] {location} — {}\n",
            error.message
        ));
        if let Some(suggestion) = &error.suggestion {
            if !suggestion.trim().is_empty() {
                out.push_str(&format!("  Suggestion: {suggestion}\n"));
            }
        }
    }

    if errors.len() > 20 {
        out.push_str(&format!("\n... and {} more errors\n", errors.len() - 20));
    }

    out
}
```

### Step 3: Call `format_compile_errors_for_prompt()` inside `build_gate_retry_context()`

Modify `build_gate_retry_context()` at line 15323 to include the structured compile errors section when `classification.compile_errors` is non-empty.

Find the current `format!()` call at line 15351 and update it:

**Current code (lines 15337-15357):**
```rust
    let classification = classify_gate_failure(classification_gate, gate_output);
    let analysis = render_failure_classification(&classification);

    let gate_excerpt = if gate_output.len() > 3000 {
        &gate_output[..3000]
    } else {
        gate_output
    };
    let agent_excerpt = if prev_agent_output.len() > 2000 {
        &prev_agent_output[..2000]
    } else {
        prev_agent_output
    };

    format!(
        "## IMPORTANT: Your previous attempt failed\n\n\
         Attempt {attempt_num} failed.\n\n\
         ### Error analysis\n{analysis}\n\n\
         ### Gate error output\n```\n{gate_excerpt}\n```\n\n\
         ### What you did last time\n```\n{agent_excerpt}\n```"
    )
```

**Updated code:**
```rust
    let classification = classify_gate_failure(classification_gate, gate_output);
    let analysis = render_failure_classification(&classification);

    // Insert a human-readable compile error table when structured errors are available.
    let compile_errors_section = if classification.compile_errors.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", format_compile_errors_for_prompt(&classification.compile_errors))
    };

    let gate_excerpt = if gate_output.len() > 3000 {
        &gate_output[..3000]
    } else {
        gate_output
    };
    let agent_excerpt = if prev_agent_output.len() > 2000 {
        &prev_agent_output[..2000]
    } else {
        prev_agent_output
    };

    format!(
        "## IMPORTANT: Your previous attempt failed\n\n\
         Attempt {attempt_num} failed.\n\n\
         ### Error analysis\n{analysis}{compile_errors_section}\n\n\
         ### Gate error output\n```\n{gate_excerpt}\n```\n\n\
         ### What you did last time\n```\n{agent_excerpt}\n```"
    )
```

### Step 4: Add unit tests for `build_gate_retry_context`

Two new tests in the existing `#[cfg(test)]` block in `event_loop.rs` (the block already has `build_gate_retry_context_compile_error_produces_analysis` and `build_gate_retry_context_truncates_long_output`):

```rust
#[test]
fn build_gate_retry_context_includes_structured_compile_errors() {
    // JSON input with an E0308 error with a suggestion.
    let json_line = r#"{"reason":"compiler-message","message":{"message":"expected `u32`, found `String`","code":{"code":"E0308","explanation":null},"level":"error","spans":[{"file_name":"src/lib.rs","line_start":42,"column_start":5,"is_primary":true}],"children":[{"message":"change `x` to `x as u32`","level":"help","spans":[]}]}}"#;
    let agent_output = "I changed the return type.";
    let result = build_gate_retry_context(json_line, agent_output, 1);

    assert!(result.contains("### Structured compile errors"));
    assert!(result.contains("E0308"));
    assert!(result.contains("src/lib.rs:42"));
    assert!(result.contains("change `x` to `x as u32`"));
}

#[test]
fn build_gate_retry_context_no_structured_section_when_no_compile_errors() {
    let gate_output = "test result: FAILED. 1 test failed";
    let agent_output = "I wrote the test.";
    let result = build_gate_retry_context(gate_output, agent_output, 1);
    assert!(!result.contains("### Structured compile errors"));
}

#[test]
fn format_compile_errors_truncates_at_20() {
    use roko_gate::{CompileError, ErrorCategory};
    let errors: Vec<CompileError> = (0..25)
        .map(|i| CompileError {
            category: ErrorCategory::Other,
            code: Some(format!("E{i:04}")),
            message: format!("error {i}"),
            file: Some("src/lib.rs".to_string()),
            line: Some(i as u32 + 1),
            column: None,
            suggestion: None,
        })
        .collect();
    let out = format_compile_errors_for_prompt(&errors);
    assert!(out.contains("### Structured compile errors"));
    assert!(out.contains("... and 5 more errors"));
    // Should not contain error 21+ directly
    assert!(!out.contains("error 20")); // 0-indexed, so error 20 is the 21st
}
```

Note: `CompileError` needs to be re-exported from `roko_gate` for these tests to compile. Check if it is currently exported at `crates/roko-gate/src/lib.rs` — if not, add `pub use compile_errors::CompileError;` there.

## Key Constraints

- **Never use `--allow-staged`** in any `cargo fix` invocation. `--allow-staged` can corrupt the staging area. Only `--allow-dirty` is permitted. This is already enforced in the existing `attempt_auto_fix()`.

- **Non-zero exit from `cargo fix` → fall through to agent**, never abort the runner. Already implemented.

- **`cargo_fix_enabled = false` must skip the auto-fix path entirely.** Already enforced at gate_dispatch.rs line 544.

- **`collect_rustc_suggestions()` must not duplicate `parse_cargo_json()` logic** — it is a narrower function that extracts only suggestion text from `children` entries.

- **The `### Structured compile errors` section must be absent** (not just empty) when `compile_errors` is empty — the condition `if classification.compile_errors.is_empty()` must produce an empty string, not an empty section header.

## Acceptance Criteria

1. `collect_rustc_suggestions(json_output)` is a public function in `crates/roko-gate`. It returns `Vec<RustcSuggestion>` and all four unit tests pass: empty, single, malformed-line, and multi-diagnostic cases.

2. `build_gate_retry_context()` includes a `### Structured compile errors` section when `classification.compile_errors` is non-empty. Each error renders as `[code / category] file:line — message` with a `Suggestion:` line when present.

3. The `### Structured compile errors` section is absent from the prompt when there are no compile errors (e.g., for a test failure that has no structured error data).

4. A prompt built from an input with 25 compile errors shows exactly 20 errors and the `... and 5 more errors` suffix.

5. `cargo fix --allow-dirty` is used for compile gate failures; `cargo clippy --fix --allow-dirty` is used for clippy gate failures. Both already implemented — verifiable by the existing unit tests for `attempt_auto_fix`.

6. `GatesConfig.cargo_fix_enabled = false` in `roko.toml` disables the auto-fix path. Already implemented and tested at gate_dispatch.rs line 2123.

## Verification Checklist

- [ ] `cargo test -p roko-gate` passes all new and existing tests
- [ ] `cargo test -p roko-cli runner::event_loop::tests` passes all new and existing tests
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean (check that `roko_gate::CompileError` is exported if used in test)
- [ ] `cargo build --workspace` builds without error
- [ ] Manually verify: trigger a compile gate failure on a known-fixable error (e.g., add an unused import `use std::collections::HashSet;` to any file, run a plan task). Check that `### Structured compile errors` appears in the retry prompt logged to stdout/stderr by the runner.
- [ ] Manually verify: for a non-compile failure (test failure), confirm `### Structured compile errors` is absent from the retry prompt.
- [ ] Run `cargo run -p roko-cli -- learn efficiency` to confirm no regressions in the efficiency log from the changes.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-gate/src/compile_errors.rs` | Add `RustcSuggestion` struct and `collect_rustc_suggestions()` function after line 425; add 4 unit tests in the `#[cfg(test)]` block |
| `crates/roko-gate/src/lib.rs` | Add `pub use compile_errors::RustcSuggestion;` and `pub use compile_errors::collect_rustc_suggestions;` if not already exported |
| `crates/roko-cli/src/runner/event_loop.rs` | Add `format_compile_errors_for_prompt()` private function before line 15323; update `build_gate_retry_context()` to call it; add 3 unit tests |
