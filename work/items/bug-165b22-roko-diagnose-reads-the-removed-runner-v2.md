+++
id = "bug-165b22"
kind = "bug"
title = "roko diagnose reads the removed Runner-v2 snapshot and fails for every Graph run"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/diagnose"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a36180342"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md#10-05"
discovered_from = "plan:portal-programme/08f-final-polish"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::build_report", "crates/roko-cli/src/commands/diagnose.rs:136"]
links = { depends_on = [], blocks = [], related = ["bug-9fc3fc", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[repro]]
command = "! (roko diagnose 08f-final-polish 2>&1 | grep -q 'No state snapshot found')"

[[verify]]
command = "grep -qE 'graph_checkpoint|GraphCheckpoint' crates/roko-cli/src/commands/diagnose.rs && cargo test -p roko-cli --bin roko commands::diagnose::tests::diagnose_reads_graph_checkpoint. This is a test the fix must add: write a failed .roko/state/graph/<plan>/checkpoint.json and assert the report shows the failed node, last error and cost; the existing test no_snapshot_gives_helpful_error must then change. The current verify is unsound for two reasons. commands/ is a bin module (main.rs:38 'mod commands;'), so 'cargo test -p roko-cli --lib commands::diagnose' matches zero tests and passes vacuously. And the literal 'state/graph' grep passes on a comment alone, while missing a fix that uses the graph_checkpoint.rs helpers."

[closed]
at = 2026-09-29
commit = "05f8854ce"
by = "roko-b6"
evidence = "roko diagnose builds its report from .roko/state/graph/<plan>/ plus costs.jsonl, gate-failures.jsonl and episodes; legacy snapshot only as fallback (05f8854ce, merged 66ad156f0). commands::diagnose tests (34) passed in the full workspace run and roko-cli lib 3042/3042 at 875482152. In main via #79."
+++

`roko diagnose <plan-id>` builds its report from `load_durable_runner_projection`, the
Runner-v2 `.roko/state/state-snapshot.json`. The Runner-v2 event loop was deleted on 2026-09-06,
and Graph runs write `.roko/state/graph/<plan>/` instead. So on any current workspace the
command fails with `No state snapshot found at .roko/state/state-snapshot.json. Run
\`roko plan run\` first.`, even right after a failed `roko plan run`. Seen on 2026-09-29 after
`08f-final-polish` failed.

The information it should report is on disk: `checkpoint.json` (status), `activities.jsonl`
(completed nodes), `costs.json`, `.roko/learn/gate-failures.jsonl` (the failing verify step and
its classification), `.roko/learn/costs.jsonl` (attempts, including timeouts) and
`.roko/episodes.jsonl`. A fix must read the Graph checkpoint for the plan and report its
failed nodes with their last error, attempts and cost. It should fall back to the legacy
snapshot only when no Graph checkpoint exists.
