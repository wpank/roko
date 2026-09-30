+++
id = "bug-c65bfe"
kind = "bug"
title = "The tool loop leaves usage_obs.source as Unknown"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "3342662ea"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-agent/src/tool_loop/agent_wrapper.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tool_loop_usage_names_its_source' crates/roko-agent/src/ && cargo test -p roko-agent --lib tool_loop_usage_names_its_source"
+++

## Problem

On bug-31438d's branch, the tool loop now sets `usage_obs.model` from the last reported model (`tool_loop/agent_wrapper.rs:313-314`), but it leaves `usage_obs.source` as `UsageSource::Unknown` (per wk-model-truth). Costs built from tool-loop usage are therefore labelled as coming from an unknown source, even when every call returned provider usage.

## Why it matters

One settled record per attempt (epic spec-b7303f): `source` says whether a cost was measured or estimated. Unknown makes a measured cost look like a guess.

## Where

The usage aggregation in `tool_loop/agent_wrapper.rs`.

## Plan

1. Set the source from the calls it sums: provider usage when every call reported usage, estimated when any call was estimated, unknown only when none reported.
2. Add `tool_loop_usage_names_its_source`.

## Done when

- [ ] Tool-loop usage carries the right source.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d's branch.
- Implemented on `work/bug-b8af02` at `3342662ea`; cargo verification deferred to the batch check. `tool_loop_usage_names_its_source` (targeted `cargo test` passed at the branch head). Each turn records whether its response reported usage (`BackendResponse::usage_source`, `ToolLoopTurnTrace.usage_source`). The loop's `usage_obs.source` is provider-reported when every turn reported, estimated when any turn was estimated or only some reported (a lower bound), and unknown when none did. `collect_stream_to_response` and `response_to_synthetic_stream` used to invent a zero usage block; they no longer do when the provider reported none.
