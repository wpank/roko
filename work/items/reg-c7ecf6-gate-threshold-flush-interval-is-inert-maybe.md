+++
id = "reg-c7ecf6"
kind = "regression"
title = "Gate-threshold flush interval is inert: maybe_flush_gate_thresholds is only called from tests"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
subsystem = ["roko-cli/gate-thresholds", "roko-gate/adaptive-thresholds"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "gaps-md#gate-threshold-flush-interval-configurable----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::GateThresholdWrites", "crates/roko-cli/src/runner/persist.rs::maybe_flush_gate_thresholds", "crates/roko-core/src/config/learning.rs:124", "crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'gate_threshold_flush_interval' crates/roko-core/src/config/learning.rs || grep -rqE 'effective_gate_threshold_flush_interval\\(\\)|maybe_flush_gate_thresholds\\(' crates/roko-cli/src/graph_execution crates/roko-cli/src/graph_task_dispatch.rs"
+++

`learning.gate_threshold_flush_interval` is still part of the schema, config layering and `config set` (`crates/roko-core/src/config/learning.rs:124`, `crates/roko-cli/src/config.rs:1168`). GAPS.md marked it RESOLVED because Runner-v2 read it once per run. After the Runner-v2 deletion, `maybe_flush_gate_thresholds` (`crates/roko-cli/src/runner/persist.rs:537`) is called only at `:1425`, `:1441` and `:1463`. All three calls are inside `mod tests` (`:1264`), so the key has no effect. It is also unconfirmed whether adaptive gate thresholds are persisted at all on the Graph path.

Fix: flush EMA gate thresholds from the Graph gate/feedback path at the configured interval, or remove the key and document the Graph behaviour.

Checked 2026-09-29 at d9e79e9d8: the key is still inert. The Graph path does persist adaptive gate thresholds: after each task's verify steps, graph_task_dispatch.rs:2216-2239 loads gate-thresholds.json, observes each step by rung and saves it every time (path wired at graph_execution/plan_runner.rs:1068). The fix is therefore to honour learning.gate_threshold_flush_interval there, or to remove the key and document the save-per-task behaviour.

## Notes

- 2026-10-01 (wk-honestbench): implemented on work/bug-730243; cargo verification deferred to the batch check.
  `GraphTaskDispatcher::new` reads `config.learning.effective_gate_threshold_flush_interval()` into a
  `GateThresholdWrites` (`graph_task_dispatch/gate_learning.rs`). `settle_gate_learning` holds each verify run's
  observations there and writes them in one locked update (`GateThresholds::update_locked`, bug-e0f472) once they add
  up to the interval, before a plan's retry budgets are read (`task_retry_budgets`), and when the dispatcher is
  dropped. The ratchet, regressions and skip advice still settle per run. Test:
  `gate_thresholds_are_written_every_flush_interval`. `maybe_flush_gate_thresholds` (persist.rs) is still called
  only from its own tests.
