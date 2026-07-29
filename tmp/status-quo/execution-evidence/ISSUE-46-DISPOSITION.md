# Issue 46 disposition — run summary contradicts plan summary

- Issue: tmp/status-quo/issues/46-RUN-SUMMARY-CONTRADICTS-PLAN-SUMMARY.md
- Disposition: **RESOLVED** by SH01-T07 (fix merged at 88b3a31)
- Merged evidence: SH01-T07 review rejection fix + original build_report rewrite

## Resolution

SH01-T07 rewrote `build_plan_report` to derive each task's category from
lifecycle/attempt state with mutually exclusive terminal categories. The fix
at 88b3a31 added:

- `TaskRunCategory` enum: Completed, Failed, Blocked, Skipped, Cancelled, Orphaned, Nonterminal
- `TaskRunSummary` struct: per-task category with reason
- `TaskPhaseDurations` struct: dispatch_ms, agent_ms, gate_ms, cleanup_ms
- `phase_durations` field on `TaskAttemptCompleted` variant
- Backward-compatible `tasks_active`/`tasks_pending` alongside new category fields in `RunTotals`/`RunCompleted`

Plan totals now reconcile with global totals: `build_plan_report` produces
per-plan `PlanReport` with per-task categorization, and `RunReport` aggregates
across plans. The `global == sum(plans)` invariant is enforced.

## Verification

```
cargo test -p roko-cli runner::event_loop   # 76 pass
cargo test -p roko-cli runner::types        # 15 pass
cargo test -p roko-cli --test e2e_self_host # 1 pass (terminal counts verified)
```
