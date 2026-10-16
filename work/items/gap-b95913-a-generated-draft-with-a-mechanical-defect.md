+++
id = "gap-b95913"
kind = "gap"
title = "A generated draft with a mechanical defect (meta.total 0, a line range past the end of its file, too many read_files) is sent back to the planner instead of repaired"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "plans-research, round v6 (tmp/hermes-roko-v6/findings/plans-research.md §3.2 and §5 item 3)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::validate_and_fix_generated_plan", "crates/roko-cli/src/plan_generate/pipeline.rs::clamp_context_ranges_past_eof", "crates/roko-cli/src/plan_policy.rs::validate_plan_budgets"]
links = { depends_on = [], blocks = [], related = ["bug-f0d60c", "gap-f8a6ea"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generated_draft_repairs_total_ranges_and_read_files' crates/roko-cli/src && cargo test -p roko-cli --lib -- generated_draft_repairs_total_ranges_and_read_files"
+++

## Problem

Every first draft in plans-research's three live runs (§3.2) was rejected for a mechanical defect that roko could
have repaired itself:
- a `read_files` range past the end of its file (`200-400` in a 71-line doc);
- `meta.total = 0`;
- 4 `read_files` where the limit is 3.

The repair step does not recount `meta.total`. It clamps only ranges that start inside the file (bug-1c75cc), and
only after the context check has already failed. It does not trim `read_files`. Each rejection costs a planner
call, and until bug-f0d60c it also cost the plan its content.

## Why it matters

Each avoidable rejection is a paid retry, and a chance for the plan to lose content (goal `golden-path`).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::validate_and_fix_generated_plan`, the extraction step that calls
  it, and `::clamp_context_ranges_past_eof`.
- `crates/roko-cli/src/plan_policy.rs::validate_plan_budgets` (PLAN_BUDGET_TOTAL, PLAN_BUDGET_READ_FILES), and
  `::fit_ranges_to_earlier_changes`, which already fits ranges at dispatch for files an earlier task changes.

## Plan

Before a generated draft is checked, repair it:
1. Set `meta.total` to its number of tasks.
2. Keep a task's first `max_read_files_per_task` `read_files`.
3. Fit line ranges in files that no earlier task changes, as dispatch already fits the others:
   - a range that ends past the end of its file is cut at the file's last line;
   - a range that starts past the end becomes an unranged entry, which shows the file within the context budget.

Print each repair.

## Done when

`[[verify]]` passes: a draft with those three defects is repaired, then validated and written after one planner
call.
