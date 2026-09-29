+++
id = "gap-ad0d39"
kind = "gap"
title = "Cost records miss Claude per-model usage and reasoning tokens, and price unknown models as Sonnet"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/claude-cli", "roko-learn/cost-table"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::parse_stream_usage", "crates/roko-learn/src/cost_table.rs:25"]
links = { depends_on = [], blocks = [], related = ["find-af6b7f", "bug-b9cb83", "find-e16c23", "bug-c30f28"], supersedes = [], duplicate_of = "" }
+++
The Claude CLI usage parser (`claude_cli_agent.rs:441-506`) reads `total_cost_usd` and `usage.{input, output, cache_creation, cache_read}`. It ignores:
- `modelUsage` (per-model usage, including subagents);
- the cost basis and the cache-write TTL split.

`reasoning_tokens` is always 0, and cost is cast to `f32`. The cost table has one cache-write rate and prices any unknown model slug at the Sonnet rate (`roko-learn/src/cost_table.rs:25-31`).

Also reported by the assessment but not re-checked here: the OpenAI-compatible path ignores cached tokens, and the built-in Opus 4.6 cache-read price is 0.25× input instead of 0.1×.

Fix: parse the full usage record, price from one dated snapshot, and flag unknown models instead of guessing.
