+++
id = "gap-9efe8e"
kind = "gap"
title = "FAST run deadline is reported as SIGTERM (exit 143) with no deadline reason"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w2-migrate"
anchors = ["crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt"]
links = { depends_on = [], blocks = [], related = ["gap-4a6dcb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'sed -n "/pub enum PlanRunInterrupt/,/^}/p" crates/roko-cli/src/graph_execution/plan_runner.rs | grep -q Deadline'
+++

When `ROKO_FAST_PLAN_DEADLINE_SECS` elapses, `fast_lane::arm_plan_deadline` fires the run's interrupt handle: the checkpoint is finalized as `interrupted` and the run exits 143, exactly as on SIGTERM (fast_lane.rs:8). `PlanRunInterrupt` has only `Interrupt` and `Terminate`, so the evidence bundle, the checkpoint and `run.completed` cannot say that the deadline, not an operator, stopped the run.

Fix: add a `Deadline` interrupt variant with its own outcome label, keeping exit 143 if dev.sh relies on it.
