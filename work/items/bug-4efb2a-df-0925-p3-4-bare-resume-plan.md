+++
id = "bug-4efb2a"
kind = "bug"
title = "Bare `--resume-plan` fails for multi-plan runs"
status = "in_progress"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs"
anchors = ["main.rs:2028", "graph_checkpoint.rs:33 DEFAULT_RUNNER_RESUME_PATH", "graph_checkpoint.rs:508", "crates/roko-cli/src/graph_checkpoint.rs::DEFAULT_RUNNER_RESUME_PATHS", "crates/roko-cli/src/main.rs:2031"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The clap default .roko/state/state-snapshot.json is not mapped to the graph checkpoint root (only executor.json is special-cased), so it is treated as a single-plan manifest and bails; the module doc claims otherwise.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs`

How to verify: Run plan run <dir> --resume-plan with no path on a multi-plan set.

Fix in progress (2026-09-28): Fixed only in the uncommitted working tree: graph_checkpoint.rs:53-56 replaces the single DEFAULT_RUNNER_RESUME_PATH (executor.json only, still at HEAD) with DEFAULT_RUNNER_RESUME_PATHS covering the clap default .roko/state/state-snapshot.json (main.rs:2031) and executor.json, and graph_checkpoint.rs:1073-1080 maps either one to the canonical .roko/state/graph directory, so a bare --resume-plan is no longer treated as a single-plan manifest (the plan_count != 1 bail is at :1086-1090). Regression tests reference both paths (graph_checkpoint.rs:2155-2157, main.rs:5373-5382). Close with the commit once the uncommitted part lands.
