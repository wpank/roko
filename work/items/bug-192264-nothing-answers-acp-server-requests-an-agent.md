+++
id = "bug-192264"
kind = "bug"
title = "Nothing answers ACP server requests: an agent waiting for permission stalls until the timeout"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/harness"]
created = 2026-10-01
updated = 2026-10-01
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
