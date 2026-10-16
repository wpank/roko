+++
id = "bug-46c04b"
kind = "bug"
title = "When the planner's model cannot run here, plan generation falls back to the first runnable model in roko.toml, which can be the cheapest"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/model_selection"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "bug-59ed90 investigation, tmp/hermes-roko-v5/findings/roko-planner.md (2026-10-15, roko main a8e27e4b7)"
anchors = ["crates/roko-cli/src/model_selection.rs::select_candidate", "crates/roko-cli/src/model_selection.rs::resolve_planner_selection"]
links = { depends_on = [], blocks = [], related = ["bug-59ed90", "bug-4e336b", "gap-1f08f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn planner_falls_back_to_the_strongest_runnable_rung' crates/roko-cli/src && cargo test -p roko-cli --lib planner_falls_back_to_the_strongest_runnable_rung"
+++

## Problem

When a workspace's planner model cannot run on this machine, plan generation falls back to the first runnable
`[models.*]` entry in file order, not to the strongest one, so a plan can be written by the cheapest model.

Found 2026-10-15 while checking plan writing for a newcomer without the `claude` CLI (bug-59ed90,
tmp/hermes-roko-v5/findings/roko-planner.md, "Planner model for a newcomer"). With `[authoring] planner_model`
unset, `resolve_planner_selection` goes to `select_candidate`: the strategist role's model, then
`[agent] default_model` when its provider runs here, else `config.models.keys().find(runnable)`. In a workspace made
by the old `roko init` (`default_model = "claude-sonnet-4-6"` on `claude_cli`) on a machine without `claude`, with
the newcomer's own `[models.*]` tables added cheapest first, the planner is the cheap one. Seen with one model:
`roko plan generate` and `/mcp` `plan_generate` both fell back to `gpt-5-4-mini` (logs in
`tmp/hermes-roko-v5/roko-planner/logs/`).

## Why it matters

Plans are written by the strongest model and run by cheap ones; a plan written by the cheapest rung is a weaker spec
for every task after it. Workspaces from the new `roko init` (bug-4e336b) set `default_model` to the strongest
runnable rung and are not affected; older ones and hand-written configs are.

## Where

- `crates/roko-cli/src/model_selection.rs::select_candidate`: the `default_model` fallback.
- `crates/roko-cli/src/model_selection.rs::resolve_planner_selection`: the planner's chain.

## Current state

Open at a8e27e4b7. The fallback is shared by every `resolve_effective_model` caller.

## Plan

1. In the planner's chain only, when `default_model` cannot run, pick the highest `[routing.ladder]` rung whose model
   can run, then the first runnable model as today; say on stderr which model plans and why.
2. A unit test with a ladder whose cheap rung is listed first.

## Done when

`[[verify]]` passes: with `default_model` on a missing CLI and runnable cheap and strong rungs, the planner is the
strong rung's model.

## Notes

- 2026-10-16 (bug-a0432b): a chat host's plan now picks its planner with
  `model_selection::chat_host_planner`, which already follows this item's order among the models roko's own tool
  loop runs: the chain's planner when it qualifies, else the strongest ladder rung that does, else the first one.
  Every other caller of the planner's chain still falls back as described above.
