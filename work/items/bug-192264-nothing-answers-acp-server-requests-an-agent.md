+++
id = "bug-192264"
kind = "bug"
title = "Nothing answers ACP server requests: an agent waiting for permission stalls until the timeout"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/harness"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-7f15df"
anchors = ["crates/roko-agent/src/harness/acp_client.rs", "crates/roko-agent/src/openclaw/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-7f15df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib acp_permission_request_is_answered"
+++

## Problem

ACP agents send server requests such as `session/request_permission`. OpenClaw logs them as auto-approved or DENIED but never replies, so an agent that waits for permission stalls until the turn timeout.

## Plan

Answer each server request with the decision logged (the spec's response shape). Add a test named `acp_permission_request_is_answered`.

## Done when

- `cargo test -p roko-agent --lib acp_permission_request_is_answered` passes.

## Notes

- Reported on 2026-10-01 by wk-guard2, working on bug-7f15df, during the evening close-out round.
- 2026-10-01 (wk-guard2): implemented on work/bug-7f15df; cargo verification deferred to the batch check.
- `AcpStdioClient::answer_server_request` answers a server request in ACP's shape: `session/request_permission` gets `{"outcome": {"outcome": "selected", "optionId": ...}}` for the offered option that matches the decision (a one-time option first), or `{"outcome": {"outcome": "cancelled"}}` when none matches; any other method gets a JSON-RPC -32601 error. OpenClaw answers with `auto_approve_permissions`, the decision it already logged. Hermes, which has no approval setting and ignored these requests, now denies them (fail closed) with a warning. Tests: `acp_permission_request_is_answered` (the replies) and `acp_permission_request_is_answered_by_hermes`, a round trip against a `bash -c` stand-in that blocks until it gets the answer.
- Left as is: requests with non-numeric JSON-RPC ids are still queued as plain notifications and get no answer, and OpenClaw's own parser reads `id`/`tool`/`arguments` rather than ACP's `toolCall`/`options`. Whether Hermes should get an `auto_approve_permissions` setting like OpenClaw's (`!require_confirmation`) is a decision for Will.
