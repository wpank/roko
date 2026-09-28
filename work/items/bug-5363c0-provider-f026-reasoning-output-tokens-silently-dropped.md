+++
id = "bug-5363c0"
kind = "bug"
title = "[provider F026] reasoning_output_tokens silently dropped in Codex provider"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F026"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F026"
anchors = ["crates/roko-agent/src/provider/codex_cli/stream.rs:228", "crates/roko-agent/src/codex_agent.rs:517"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib reasoning_tokens_do_not_inflate_output_totals"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8 (landed in 72e0a76b8): crates/roko-agent/src/provider/codex_cli/stream.rs:75-78 parses reasoning_output_tokens and :224-228 forwards it as AgentRuntimeEvent::TokenUsage.reasoning_tokens. The count is a subset of output_tokens, which estimate_codex_cost already bills at the output rate (test reasoning_tokens_do_not_inflate_output_totals, :425). Nit: the HTTP CodexAgent (crates/roko-agent/src/codex_agent.rs:517) still reports reasoning_tokens: None, but its cost comes from completion_tokens, which include reasoning."
+++
The Codex adapter parses `reasoning_output_tokens` from the API response but does not store it in `UsageObservation`. These tokens represent real cost (charged at output rate) that is not tracked.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F026`

How to verify: Confirm in crates/roko-agent/src/provider/ whether still true: `reasoning_output_tokens` silently dropped in Codex provider

Verified 2026-09-28: fixed; see `[closed].evidence`.
