# 114 — Diagnose Command Enrichment (3 Missing Fields)

**Priority**: P2 — The `roko diagnose` command exists and is structurally correct but is missing three fields that make it fully useful for automated debugging: per-error classification, episode IDs, and plan-level cost.
**Size**: XS (2-3 hours)
**Crates**: `crates/roko-cli/src/commands/diagnose.rs`
**Depends on**: None (the command already exists)
**Sources**: `tmp/backlog/_checklist-gaps.md` §0.5, `tmp/backlog/_mori-old-gaps.md` MO-03

---

## Background

`roko diagnose <plan-id>` was implemented (the file `crates/roko-cli/src/commands/diagnose.rs` exists as an untracked new file as of 2026-08-19). It produces a `DiagnoseReport` with `FailedTaskInfo`, `GateResultInfo`, `RunStateSummary`, and `GitStateInfo` — enough to understand task status, git state, and basic run summary.

Three fields from the specification are missing. First, per-error classification: the implementation checklist specifies that each error entry in gate output should carry `error_class` (compile_error, test_failure, lint_warning, etc.), `error_summary` (one-line human-readable), and `suggestion` (what to try next). Without this, consumers must parse raw compiler output themselves. Second, episode IDs linked to the failing task are absent — agents need these to retrieve full context from `.roko/episodes.jsonl`. Third, the plan-level `total_cost_usd` is only accessible via the `RunStateSummary` aggregation and not surfaced as a top-level field.

These are additive fields that do not require structural changes to the existing report.

## Current State

- `crates/roko-cli/src/commands/diagnose.rs` — exists with `DiagnoseReport`, `FailedTaskInfo`, `GateResultInfo`, `RunStateSummary`, `GitStateInfo`.
- The `classified_errors` field is NOT present on `GateResultInfo`.
- The `episode_ids: Vec<String>` field is NOT present on `FailedTaskInfo`.
- The `total_cost_usd: Option<f64>` field is NOT present as a top-level `DiagnoseReport` field.
- Wire-up into `main.rs` needs verification (see #111 for pattern).

## Implementation Plan

1. **Add `classified_errors` to `GateResultInfo`**:
   ```rust
   pub struct ClassifiedError {
       pub error_class: ErrorClass,   // enum: CompileError, TestFailure, LintWarning, LinkError, RuntimePanic, Unknown
       pub file: Option<String>,
       pub line: Option<u32>,
       pub error_summary: String,
       pub suggestion: Option<String>,
   }
   ```
   Parse gate output (which is stdout/stderr from `cargo build` / `cargo test` / `cargo clippy`) using the existing error classification logic from `roko-gate`'s diagnostic utilities.

2. **Add `episode_ids: Vec<String>` to `FailedTaskInfo`**: Query `.roko/episodes.jsonl` filtering on `task_id` matching the failed task. Return the last 5 episode IDs for that task.

3. **Add `total_cost_usd: Option<f64>` to `DiagnoseReport`**: Sum cost from `RunStateSummary` or efficiency events for the plan. Derive from `.roko/learn/efficiency.jsonl` filtered by plan ID.

4. **Verify `main.rs` registration**: Confirm `roko diagnose` appears in the top-level command dispatch table. If not, add it.

5. **Update `DiagnoseReport` serialization**: Ensure the new fields appear in `--json` output (the command already outputs JSON by default per spec).

## Acceptance Criteria

1. `roko diagnose <plan-id>` output includes `classified_errors` array in each gate result with at least `error_class` and `error_summary` per entry.
2. Each failed task entry includes `episode_ids` listing relevant episode IDs from `.roko/episodes.jsonl`.
3. `DiagnoseReport` top-level includes `total_cost_usd` (may be null if no efficiency data exists).
4. The command is reachable via `roko diagnose --help` without error.
5. Output is valid JSON.

## Verification Checklist

- [ ] Run a plan that fails a compile gate; verify `roko diagnose <id>` includes `classified_errors` with `error_class: "compile_error"`.
- [ ] Verify `episode_ids` is a non-empty array for a plan that ran at least one agent turn.
- [ ] Verify `total_cost_usd` is present (even if 0.0) in the JSON output.
- [ ] Run `roko diagnose --help` and verify the command is registered.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/diagnose.rs` | Add `ClassifiedError`, `episode_ids`, `total_cost_usd` fields; update parsing logic |
| `crates/roko-cli/src/main.rs` | Verify or add `roko diagnose` top-level dispatch |
