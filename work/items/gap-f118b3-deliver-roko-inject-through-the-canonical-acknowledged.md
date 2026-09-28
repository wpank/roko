+++
id = "gap-f118b3"
kind = "gap"
title = "Deliver `roko inject` Through the Canonical Acknowledged Control Transport"
status = "open"
triage = "verified"
severity = "p1"
goal = "features"
subsystem = ["roko-cli/inject"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/361-inject-acknowledged-control-transport.md#361 — Deliver `roko inject` Through the Canonical Acknowledged Control Transport"
discovered_from = "audit:tmp/backlog/archive/361-inject-acknowledged-control-transport.md#361 — Deliver `roko inject` Through the Canonical Acknowledged Control Transport"
anchors = ["crates/roko-cli/src/commands/util.rs::cmd_inject"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'DaemonControlAckV1' crates/roko-cli/src"
+++
[blocked] Blocked on #233, #255, and #325 —

Imported without verification from:
- `tmp/backlog/archive/361-inject-acknowledged-control-transport.md#361 — Deliver `roko inject` Through the Canonical Acknowledged Control Transport`

Warning: every file this item cites is gone (`.roko/daemon.sock`, `.roko/run/roko-<session>.sock`, `tmp/cli-audit/14-status-replay-inject.md`) — likely obsolete or moved.

How to verify: Check: Live receiver observes exact kind/payload/session once.; Success is impossible without a matching executor acknowledgement.; Duplicate request IDs do not duplicate delivery. [evidence: own status: Blocked on #233, #255, and #325]

Verified 2026-09-28: still true. cmd_inject (crates/roko-cli/src/commands/util.rs:1556) writes the payload file (:1597) and a control command into the state dir (:1610), then returns EXIT_SUCCESS (:1630) without waiting for any executor acknowledgement. No DaemonControlRequestV1 or DaemonControlAckV1 type and no request-ID dedup exist anywhere. Subsystem corrected from roko-neuro to roko-cli/inject. The status of blockers #233, #255 and #325 was not checked.
