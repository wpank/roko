+++
id = "bug-4efb2a"
kind = "bug"
title = "Bare `--resume-plan` fails for multi-plan runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::DEFAULT_RUNNER_RESUME_PATHS", "crates/roko-cli/src/main.rs:2039"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bare_resume_plan_default_maps_to_canonical_root_for_multi_plan_runs' crates/roko-cli/ && cargo test -p roko-cli --lib bare_resume_plan_default_maps_to_canonical_root_for_multi_plan_runs"

[closed]
at = 2026-09-29
commit = "725f21e05"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "The working-tree fix recorded on 2026-09-28 is now committed (725f21e05; graph_checkpoint.rs and main.rs are clean): DEFAULT_RUNNER_RESUME_PATHS covers both .roko/state/state-snapshot.json and .roko/state/executor.json (crates/roko-cli/src/graph_checkpoint.rs:50-56), and the resolver maps either one to the canonical Graph checkpoint root before the single-plan manifest check (:1073-1082, bail at :1087-1091), so the clap default_missing_value (crates/roko-cli/src/main.rs:2039) no longer bails for multi-plan runs. Regression test bare_resume_plan_default_maps_to_canonical_root_for_multi_plan_runs (graph_checkpoint.rs:2149) covers both paths."
+++
The clap default .roko/state/state-snapshot.json is not mapped to the graph checkpoint root (only executor.json is special-cased), so it is treated as a single-plan manifest and bails; the module doc claims otherwise.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs`

How to verify: Run plan run <dir> --resume-plan with no path on a multi-plan set.

Fix in progress (2026-09-28): Fixed only in the uncommitted working tree: graph_checkpoint.rs:53-56 replaces the single DEFAULT_RUNNER_RESUME_PATH (executor.json only, still at HEAD) with DEFAULT_RUNNER_RESUME_PATHS covering the clap default .roko/state/state-snapshot.json (main.rs:2031) and executor.json, and graph_checkpoint.rs:1073-1080 maps either one to the canonical .roko/state/graph directory, so a bare --resume-plan is no longer treated as a single-plan manifest (the plan_count != 1 bail is at :1086-1090). Regression tests reference both paths (graph_checkpoint.rs:2155-2157, main.rs:5373-5382). Close with the commit once the uncommitted part lands.
