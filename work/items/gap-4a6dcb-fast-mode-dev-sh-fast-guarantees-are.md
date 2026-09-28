+++
id = "gap-4a6dcb"
kind = "gap"
title = "FAST mode (dev.sh fast) guarantees are only partly ported to the Graph engine"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "session:roko-b6 message 2026-09-28 (commit 725f21e05)"
discovered_from = "item:bug-f7943a"
anchors = ["crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline", "dev.sh:225", "crates/roko-cli/src/runner/agent_stream.rs:705", "crates/roko-cli/src/runner/gate_dispatch.rs:907"]
links = { depends_on = [], blocks = [], related = ["bug-f7943a"], supersedes = [], duplicate_of = "" }
+++

`./dev.sh fast` runs on the Graph engine again (bug-f7943a), but only part of FAST mode was ported.
According to dev.sh's own help (lines 225-227), on the Graph engine `ROKO_FAST_MODE` bounds prompt context,
enforces the one-verify plan contract and stops the run at `ROKO_FAST_PLAN_DEADLINE_SECS`. The other
variables are only recorded in the evidence metadata. The session that ported the rest lists these as
still missing on the Graph path:

- the patch-only prompt section (tell the provider to hand off after patching);
- the 90 s attempt/silence clamp;
- the 6-turn agent cap (`ROKO_FAST_MAX_AGENT_TURNS` is read only in `runner/agent_stream.rs`);
- the `dev-fast` cargo profile for gates;
- no-autofix.

CLAUDE.md's FAST paragraph still describes the full set, so it overstates what FAST does today.
Done when each feature works on the Graph path and has a test, or is dropped from dev.sh's help and CLAUDE.md.
