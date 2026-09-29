+++
id = "gap-644040"
kind = "gap"
title = "No way to run with learning frozen: prompts and routing change from run to run"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:3061", "crates/roko-cli/src/dispatch/prompt_builder.rs:152", "crates/roko-cli/src/graph_task_dispatch.rs:2458", "crates/roko-cli/src/graph_task_dispatch.rs:2990"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`[meta] skip_enrichment` suppresses only eval artifacts and routing advice (`graph_task_dispatch.rs:3061-3074`). Every run still reads and writes learned state:
- c-factor context, section effectiveness, knowledge, episode knowledge and playbooks are injected into prompts (`dispatch/prompt_builder.rs`);
- post-gate reflections are persisted for later attempts (`graph_task_dispatch.rs:2458-2505`);
- T0 reflexes can replace dispatch for tasks with no verify step (`:2990-3059`);
- the router and knowledge stores update after each run;
- dreams and `roko serve` timers keep changing knowledge in the background.

Benchmarks and A/B comparisons therefore cannot hold learning fixed, and repeated seeds see each other's traces.

Fix: a frozen-learning mode (learned state read-only, no writes, dreams and timers off), recorded in each run's manifest.
