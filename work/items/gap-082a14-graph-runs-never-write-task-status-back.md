+++
id = "gap-082a14"
kind = "gap"
title = "Graph runs never write task status back to tasks.toml"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/02-artifact-inventory.md"
discovered_from = "doc:tmp/work-management/02-artifact-inventory.md"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs", "plans/portal-programme/01-backend-plan-service/tasks.toml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Task `status` fields in `tasks.toml` are authored values the Graph engine never updates; completion lives only in `.roko/state/graph/<plan>/checkpoint.json`.
Example: `portal-programme/01-backend-plan-service` has a `succeeded` checkpoint while all its tasks still read `status = "ready"`.
Plan listings, indexes and `backlog audit` that read manifests therefore report finished plans as not started.
Fix: write terminal status back atomically after the checkpoint commits, or make every reader derive status from checkpoints; document which is canonical.
