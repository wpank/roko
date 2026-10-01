+++
id = "bug-06e2d1"
kind = "bug"
title = "DF-0925 P3-2: Resume replays force-accepted tasks as genuine successes"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-graph/resume"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-2. Resume would launder a force-accepted task into a success"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-2. Resume would launder a force-accepted task into a success"
anchors = ["crates/roko-graph/src/replay.rs::ActivityReplayer", "crates/roko-cli/src/graph_checkpoint.rs:225", "crates/roko-graph/src/cells/task_executor.rs::TaskGateVerdict"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-graph --lib forced_accept_record"

[[verify]]
command = "cargo test -p roko-cli resume_reruns_unverified_or_forced_task_records"

[closed]
at = 2026-09-28
commit = "725f21e05"
evidence = "Committed in 725f21e05 by the portal session. Re-checked 2026-09-28: the cited files are clean at HEAD and the named symbols and tests exist there (not rebuilt or retested here). task outputs carry a TaskGateVerdict stamp incl. ForcedAccept (crates/roko-graph/src/cells/task_executor.rs); crates/roko-graph/src/replay.rs never replays forced-accept records (tests forced_accept_record_is_never_replayed_and_can_be_superseded, forced_accept_record_alone_forces_re_execution); crates/roko-cli/src/graph_checkpoint.rs:225 re-executes unverified/force-accepted nodes on resume (test resume_reruns_unverified_or_forced_task_records)."
+++
activities.jsonl records only successful outputs and resume marks those nodes Complete without re-verification; force-accept state lived only in memory, so a BLOCK verdict would be replayed as success.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-2. Resume would launder a force-accepted task into a success`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-7. Resume works, and it is fast`

How to verify: Force-accept a task, interrupt, resume; check node re-verification.

Verified 2026-09-28: fixed in the uncommitted working tree (replay.rs:460/495 tests, graph_checkpoint.rs:225 + test at :1159, task_executor.rs TaskGateVerdict::ForcedAccept). Closure depends on that work being committed.
