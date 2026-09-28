+++
id = "gap-97ce62"
kind = "gap"
title = "Shared-run reports mislabel total tokens as input_tokens (no input/output split in AgentCompleted)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/shared_runs"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-serve/src/routes/shared_runs.rs:477"
discovered_from = "audit:crates/roko-serve/src/routes/shared_runs.rs:477"
anchors = ["crates/roko-serve/src/routes/shared_runs.rs", "roko_runtime::RuntimeEvent::AgentCompleted"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
TODO: RuntimeEvent::AgentCompleted only carries total tokens_used, so shared-run reports put the total into input_tokens and leave output_tokens None. Needs an input/output breakdown on the event.

Imported without verification from:
- `crates/roko-serve/src/routes/shared_runs.rs:477`
- `crates/roko-serve/src/routes/shared_runs.rs:636`

How to verify: Check RuntimeEvent::AgentCompleted fields; check whether providers already report prompt/completion tokens upstream.
