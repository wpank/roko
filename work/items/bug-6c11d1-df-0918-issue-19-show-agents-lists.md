+++
id = "bug-6c11d1"
kind = "bug"
title = "`show agents` lists long-gone agents"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/show"]
created = 2026-09-18
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-19: `roko show agents` shows stale agents from months ago"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-19: `roko show agents` shows stale agents from months ago"
anchors = ["crates/roko-cli/src/commands/show.rs::agent_rows"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn show_agents_marks_or_omits_stale_agents' crates/roko-cli/src/commands/show.rs && cargo test -p roko-cli --bin roko show_agents_marks_or_omits_stale_agents"
+++
Agents from May/August (H11:12, DEMO-T01:1) still appear; needs age filtering or stale marking.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-19: `roko show agents` shows stale agents from months ago`

How to verify: Run roko show agents; check timestamps.

Verified 2026-09-28 (static check against 3d0ee4d02): agent_rows (crates/roko-cli/src/commands/show.rs:861-890) lists every agent in the durable projection plus every distinct agent_id in the full .roko/learn/efficiency.jsonl, read with no age limit (tui/dashboard.rs:564). Neither source is filtered by age or marked stale. The 'last <timestamp>' label comes from or_insert_with (show.rs:874-886), so it shows the first event seen for that agent, not the most recent one. No commit or uncommitted change to show.rs addresses this.

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `roko show` takes `--since WHEN` (a span such as 24h or 7d, YYYY-MM-DD, RFC 3339, or `all`; default 7d).
  `agent_rows` keeps each efficiency agent's latest event (it kept the first), lists Runner-projection agents
  and then agents active in the window, newest first, and prints one line counting the older agents it hides.
- The verify command ran `--lib`, but `commands/show.rs` belongs to the `roko` binary, so the filter matched
  nothing and passed vacuously. It now runs `--bin roko`.
