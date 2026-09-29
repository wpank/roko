+++
id = "find-d1a883"
kind = "finding"
title = "Plan-set footprints trust declared files and read verify commands word by word"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph_execution/plan_set"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/graph_execution/plan_set.rs::built_by"]
links = { depends_on = [], blocks = [], related = ["gap-0001a1"], supersedes = [], duplicate_of = "" }
+++

The plan-set scheduler decides which plans may run together from each plan's footprint: the files its tasks declare, plus the areas its verify commands build. `AreaResolver::built_by` splits each command with `split_whitespace` and matches words such as `-p <crate>`. A task that edits files it did not declare, or a verify command that builds more than its words show (a script, or an alias for `cargo test --workspace`), can let two conflicting plans run side by side. Undeclared files and workspace manifests already make a plan run alone (test `undeclared_files_and_workspace_manifests_run_alone`).

Decide whether to also compare the files each task actually changed, and stop co-scheduling on overlap, instead of trusting declarations.
