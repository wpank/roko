+++
id = "gap-7147bb"
kind = "gap"
title = "Resume rejects checkpoints after a --max-tasks change, and `roko resume` cannot pass --max-tasks"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_checkpoint", "roko-cli/resume"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3b-fingerprint"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::convert_plan", "crates/roko-graph/src/fingerprint.rs::EXECUTION_META_FIELDS", "crates/roko-cli/src/main.rs:1028"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'fn resume_accepts_a_different_max_tasks' crates/roko-cli/src && cargo test -p roko-cli --lib resume_accepts_a_different_max_tasks"
+++

The fingerprint no longer changes with the binary (w3b), but it still covers run policy. `--max-tasks` becomes the graph's `max_parallel` (graph_checkpoint.rs:1477), and both the graph policy and the `max_parallel` meta field are fingerprinted (roko-graph fingerprint.rs:20-41). Resuming with a different `--max-tasks` than the original run is therefore refused as a different graph. `roko resume` takes only a run id and `--workdir` (main.rs `Command::Resume`), so a run started with `--max-tasks` cannot be resumed through it at all. w3b also reports that checkpoints written before the fingerprint fix used a different `TaskDef` serialization and cannot be migrated.

Fix: leave scheduling policy out of the fingerprint (or store it in the checkpoint and reuse it on resume), and let `roko resume` reuse the original run's options.
