+++
id = "gap-dcae09"
kind = "gap"
title = "No process-level crash/restart equivalence proof for Graph plan runs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/resume"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#remaining-work-estimate/tranche-3"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/graph_checkpoint.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Graph checkpoints, Activity replay and cost-ledger recovery have unit coverage. No process-level test kills a real `roko plan run` at defined points and checks that the resumed run reaches the same terminal state: no duplicate side effects, the same cost ledger and the same delivered commits. Deterministic self-host repair after a crash is also unproven. GAPS.md listed this as remaining-work tranche 3.

Fix: add a kill-point matrix harness that runs the CLI as a subprocess against a fixture plan and compares the terminal state after resume.
