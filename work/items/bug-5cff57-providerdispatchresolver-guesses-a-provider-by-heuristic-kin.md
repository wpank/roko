+++
id = "bug-5cff57"
kind = "bug"
title = "ProviderDispatchResolver guesses a provider by heuristic kind for an unknown model key"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-fd44df"
anchors = ["crates/roko-cli/src/dispatch/"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-fd44df"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib provider_dispatch_resolver_unknown_model"
+++

## Problem

gap-fd44df made `create_agent_for_model` error on an unknown model key with no command. roko-cli's `ProviderDispatchResolver::resolve` still picks a provider by a heuristic on the key, so an unknown key can dispatch to the wrong provider.

## Plan

Resolve through `try_resolve_model`, and fail with a clear error for unknown keys. Add a test named `provider_dispatch_resolver_unknown_model_*`.

## Done when

- `cargo test -p roko-cli --lib provider_dispatch_resolver_unknown_model` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-fd44df, during the evening close-out round.
