+++
id = "bug-165b22"
kind = "bug"
title = "roko diagnose reads the removed Runner-v2 snapshot and fails for every Graph run"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/diagnose"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md#10-05"
discovered_from = "plan:portal-programme/08f-final-polish"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::build_report", "crates/roko-cli/src/commands/diagnose.rs:136"]
links = { depends_on = [], blocks = [], related = ["bug-9fc3fc", "bug-230de6"], supersedes = [], duplicate_of = "" }

[[repro]]
command = "! (roko diagnose 08f-final-polish 2>&1 | grep -q 'No state snapshot found')"

[[verify]]
command = "grep -q 'state/graph' crates/roko-cli/src/commands/diagnose.rs && cargo test -p roko-cli --lib commands::diagnose"
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
