# CLI Output Redesign

**Priority**: P2 — developer experience, CLI feels raw and unpolished
**Size**: L (3-5 days)
**Crate**: `crates/roko-cli/src/`

---

## Problem

CLI output has no structured reporting layer. All output goes through ad-hoc `println!`,
`eprintln!`, and `tracing` calls with no consistent formatting, progress indication, or
color system. There is no `CliReporter` trait, no spinner integration, no color system,
no multi-progress tracking for plan runs, and no error deduplication.

Concrete symptoms:
- `roko plan run` shows a wall of unstructured text with no progress indication
- The same provider failure message repeats N times with no dedup
- There is no way to get machine-parseable output (JSON) for scripting
- Debug tracing logs are interleaved with user-facing output
- Success/failure states have no visual differentiation (no colors, no icons)

---

## Section A: Current State

**A1.** Search `crates/roko-cli/src/` for `println!`, `eprintln!`, and `print!` to see
the scope. Key files with heavy ad-hoc output include `run.rs`, `agent_exec.rs`,
`plan_validate.rs`, and command modules under `commands/`.

**A2.** The `redesign-plan.md` (Phase 14) had a detailed 6-part specification for this
work. Search `tmp/` for this document if it still exists — it contains the original
design intent.

**A3.** The TUI (`crates/roko-cli/src/tui/`) has its own rendering via ratatui, which is
separate from this work. This spec covers the non-TUI CLI output only.

---

## Section B: Define the CliReporter Trait

**B1.** Define a `CliReporter` trait with these methods:

```rust
trait CliReporter {
    fn status(&self, msg: &str);           // Informational status line
    fn progress(&self, current: u64, total: u64, msg: &str);  // Progress update
    fn success(&self, msg: &str);          // Green checkmark + message
    fn warning(&self, msg: &str);          // Yellow warning + message
    fn error(&self, msg: &str);            // Red error + message
    fn section(&self, title: &str);        // Section header
    fn table(&self, headers: &[&str], rows: &[Vec<String>]);  // Tabular data
}
```

**B2.** Place this trait in a new module within `crates/roko-cli/src/` (e.g.,
`reporter.rs` or `output.rs`).

---

## Section C: Implement Concrete Reporters

**C1.** `HumanReporter` — the default. Uses owo-colors (or similar) for consistent
color theming and indicatif for spinners and progress bars. Output goes to stderr for
status/progress, stdout for final results.

**C2.** `JsonReporter` — for `--format json`. Each event is a single JSON line on stdout
with fields like `{"type": "status", "message": "...", "timestamp": "..."}`. No colors,
no spinners.

**C3.** `QuietReporter` — for `--quiet`. Only emits errors and the final result. No
progress, no status, no warnings.

---

## Section D: Multi-Progress for Plan Runs

**D1.** `roko plan run` should show one progress bar per task using indicatif's
`MultiProgress`. Each bar shows the task name, current state (pending/running/gated/done),
and elapsed time.

**D2.** The overall plan progress should show as a top-level bar (e.g.,
"Plan: 3/7 tasks complete").

**D3.** When a task fails a gate, the bar should turn red and show the failure reason
inline rather than dumping a wall of text.

---

## Section E: Error Deduplication

**E1.** Repeated identical error messages (e.g., the same provider timeout shown for
every retry) should be collapsed. Show the first occurrence in full, then
"(repeated N more times)" for subsequent identical errors.

**E2.** At the end of a plan run, show a summary of all unique errors with counts.

---

## Section F: Separate Tracing from User Output

**F1.** `tracing` output (debug/info/warn) should go to a log file or stderr only when
`--verbose` / `-v` is passed. By default, only user-facing output (via the `CliReporter`)
should be visible.

**F2.** This requires auditing the existing `tracing` subscriber setup to ensure log
levels and output destinations are correct.

---

## Acceptance criteria

- [ ] `CliReporter` trait defined with `status`, `progress`, `success`, `warning`, `error`, `section`, `table` methods
- [ ] `HumanReporter` implemented with colored output and spinners
- [ ] `JsonReporter` implemented producing one JSON object per line
- [ ] `QuietReporter` implemented showing only errors and final result
- [ ] `roko plan run` shows multi-progress bars (one per task, one overall)
- [ ] Repeated identical errors are collapsed with a count
- [ ] `--format json` flag produces machine-parseable output
- [ ] `--quiet` flag suppresses all non-error, non-result output
- [ ] Tracing debug logs do not appear in default CLI output
- [ ] Existing `cargo test -p roko-cli` passes with no regressions
- [ ] Manual verification: `roko plan run` shows structured progress; `--format json` is parseable by `jq`; `--quiet` is minimal

### Not in scope
- TUI rendering changes (ratatui is a separate subsystem)
- Serve-side output formatting (HTTP responses have their own serialization)
- Changing tracing instrumentation within non-CLI crates
- Internationalization / localization of output strings
