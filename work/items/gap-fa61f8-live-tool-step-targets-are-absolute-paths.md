+++
id = "gap-fa61f8"
kind = "gap"
title = "Live tool-step targets are absolute paths for real providers, so every step carries the full workspace path"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-agent/live-output"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-agent/src/live_output.rs::tool_step_target"]
links = { depends_on = [], blocks = [], related = ["find-0d280d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tool_step_target_is_workspace_relative' crates/roko-agent/ && cargo test -p roko-agent tool_step_target_is_workspace_relative"
+++

## Problem

In the 09 real-model run, the four live tool steps carried these targets:
`/private/tmp/roko-hello-JdC7dN/roko.toml` (Read), `/private/tmp/roko-hello-JdC7dN` (Grep),
`/private/tmp/roko-hello-JdC7dN/Cargo.toml` and `/private/tmp/roko-hello-JdC7dN/src/main.rs` (Write)
(`tmp/portal-audit/evidence/hello-world-real-run1/events.sse`).

`tool_step_target` takes `file_path` or `path` verbatim, then scrubs secrets and truncates to 120
characters, and Claude Code passes absolute paths. The design and the fake agent show
workspace-relative targets (`hello/main.rs`). Absolute ones push the file name toward the
120-character cut, and put the absolute workspace path, normally under the user's home directory,
in every step published on the event stream.

## Why it matters

Goal `visibility`. The live step is the portal's main "what is the agent doing" signal (08c, contract
§3.5).

## Where

`crates/roko-agent/src/live_output.rs::tool_step_target`, and its callers, which know the dispatch
working directory.

## Plan

Strip the task's working-directory prefix (canonicalized) from the target, and keep paths outside
the workspace absolute.

## Done when

A roko-agent test `tool_step_target_is_workspace_relative` passes. The `[[verify]]` runs it.

## Notes

Found by plan 09 T04 (see VERDICT).
