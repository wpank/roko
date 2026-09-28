+++
id = "find-43768e"
kind = "finding"
title = "DF-0926 N-2: Default agent_dispatch_secs=600 too low for build-heavy tasks"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds"
anchors = ["timeouts.agent_dispatch_secs", "graph_task_dispatch.rs:1883"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Tasks that build and verify exceed 600s, fail with error_class=Timeout and burn the full retry budget (3 x 600s) learning nothing.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-2. `timeouts.agent_dispatch_secs = 600` is too low for any task that builds`

How to verify: Review default and timeout-retry policy.
