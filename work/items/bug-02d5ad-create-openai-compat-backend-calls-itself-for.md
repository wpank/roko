+++
id = "bug-02d5ad"
kind = "bug"
title = "create_openai_compat_backend calls itself for Hermes and OpenClaw, and its limiter variant has no callers"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-53088d"
anchors = ["crates/roko-agent/src/tool_loop/backends/mod.rs::create_openai_compat_backend", "crates/roko-agent/src/tool_loop/backends/mod.rs::create_openai_compat_backend_with_limiter"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-53088d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib openai_compat_backend_for_hermes"
+++

## Problem

`create_openai_compat_backend` (`tool_loop/backends/mod.rs`) calls itself for the Hermes and OpenClaw kinds, which recurses without end if `create_tool_loop_backend` routes either kind there. `create_openai_compat_backend_with_limiter` has no callers.

## Plan

Route Hermes and OpenClaw to their real backends, or return an error, and delete or wire the limiter variant. Add a test named `openai_compat_backend_for_hermes_*`.

## Done when

- `cargo test -p roko-agent --lib openai_compat_backend_for_hermes` passes.

## Notes

- Reported on 2026-10-01 by the worker on bug-53088d, during the evening close-out round.
