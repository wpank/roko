+++
id = "bug-35379d"
kind = "bug"
title = "Provider failover silently runs a different model and records it as if it had been chosen"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch/failover.rs::run_bridge_with_failover", "crates/roko-cli/src/graph_task_dispatch/failover.rs::failover_model", "roko.toml:282"]
links = { depends_on = [], blocks = [], related = ["find-229e9c", "bug-f68404"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'planned_model|substituted_from' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-learn/src/efficiency.rs && cargo test -p roko-cli --lib failover_records_planned_and_substitute_model"
+++
When the planned provider is blocked or reports usage exhaustion, `run_bridge_with_failover` runs `failover_model()` instead. The substitution is only logged (`tracing::warn!("model substitution: …")`, `graph_task_dispatch.rs:4600`).
- `dispatch.target` names the model that actually ran, and nothing on the attempt, episode or efficiency record marks it as a substitute.
- The router then credits the substitute as its own pick.
- This repo's `roko.toml:282` falls back to kimi-k2-5, glm51 and gpt-4o, so a task planned for one model can be finished by another without a trace.
- Pinning with `--model` turns failover into an error.

Fix: record the planned model, the substitute and the reason on every attempt record; keep substituted attempts out of router credit, or credit them explicitly; allow failover to be switched off per run.

2026-09-29: re-verified at d9e79e9d8. Unchanged: substitution is still only logged (failover_model WARN at graph_task_dispatch.rs:4971, formerly ~4600); no record field for the planned model or substitution reason and no per-run opt-out.
