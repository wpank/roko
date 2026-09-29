+++
id = "bug-fade34"
kind = "bug"
title = "`knowledge query` results dump full hypothesis text"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/knowledge"]
created = 2026-09-18
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy"
anchors = ["crates/roko-cli/src/commands/knowledge.rs::cmd_neuro", "crates/roko-cli/src/main.rs:1171"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n -A12 'Query the durable knowledge store' crates/roko-cli/src/main.rs | grep -q verbose && grep -n -A60 'NeuroCmd::Query { topic, workdir' crates/roko-cli/src/commands/knowledge.rs | grep -qE 'truncat|lines\\(\\)\\.take'"
+++
Results print full strategy hypothesis text; proposal: truncate to ~2 lines with --verbose for full output.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-18: `roko knowledge query` results are verbose/noisy`

How to verify: Run a knowledge query and inspect output width.

Verified 2026-09-29 at d9e79e9d8. The text-mode output of roko knowledge query (crates/roko-cli/src/commands/knowledge.rs, cmd_neuro, NeuroCmd::Query arm) prints the full entry.content for every result, and KnowledgeCmd::Query in main.rs has no --verbose flag. On the same path, the declared --limit flag is ignored: dispatch_knowledge hardcodes limit: 10 and cmd_neuro calls store.query(&topic, 10).
