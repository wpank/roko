+++
id = "gap-386329"
kind = "gap"
title = "Plan-set entries carry no group or directory, so the F1 plan tree cannot group plans"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-core/dashboard_snapshot", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::PlanSetEntry", "crates/roko-cli/src/tui/views/dashboard_view.rs", "crates/roko-cli/src/tui/widgets/plan_tree.rs::render_plan_tree"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^pub struct PlanSetEntry/,/^}/p' crates/roko-core/src/dashboard_snapshot.rs | grep -qE 'pub (group|dir)' && grep -rqw 'fn plan_tree_groups_plans_by_plan_set' crates/roko-cli/src/tui/widgets/plan_tree.rs && cargo test -p roko-cli --lib plan_tree_groups_plans_by_plan_set"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:29Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:13:11Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`PlanSetEntry` (the `plan_set_loaded` payload and the snapshot's `plan_set`) has `plan_id`, `title`, `tasks_total`, `wave`, `depends_on` and `conflicts_with`, but not the plan's group (programme directory) or path. The F2 Plans view now groups by wave (w3a), but the F1 dashboard plan tree lists a nested programme such as `plans/portal-programme/*` flat, and other clients cannot group by directory either (also reported by w3a).

Fix: add the group or relative directory to `PlanSetEntry` and group the F1 tree by it.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. The portal rail already groups plans using the disk plan listing's group field (apps/portal/src/lib/planRows.ts:139), so the gap is limited to plan_set_loaded and snapshot consumers such as the F1 tree.

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `PlanSetEntry` has `group: Option<String>` (skipped on the wire when `None`, so older readers and the wire-shape
  test are unaffected). `graph_execution/plan_runner.rs::plan_set_entries` fills it from each plan's directory
  relative to the workspace `plans/` (as plan discovery names it). `TuiState` keeps the announced `plan_set`, and
  `TuiState::plan_group` reads the plan set's group, then disk discovery. The F1 tree (rendered by
  `tui/widgets/plan_tree.rs`, which `dashboard_view.rs` calls) groups plans under a `◆ <group> (done/total)` header
  when the listed plans span several sets, inside each wave or in the flat list, and names the set in the title
  when they all share one. F2's synthesized summaries also take the plan set's group first.
- The old verify's second clause grepped `dashboard_view.rs` for `.group`, which already matched a test line
  (`PageId::Learning.group()`), so it passed before the fix. It now runs the named tree test.
