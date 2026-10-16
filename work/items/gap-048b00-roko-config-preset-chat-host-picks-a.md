+++
id = "gap-048b00"
kind = "gap"
title = "roko config preset chat-host picks a Cerebras data model without saying that its rate limits can withhold tool output from the agent"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "hermes-roko round v5, the round's lead and roko-demos, 2026-10-15"
anchors = ["crates/roko-cli/src/chat_host.rs::plan_chat_host", "crates/roko-cli/src/commands/tune.rs::cmd_chat_host_preset"]
links = { depends_on = [], blocks = [], related = ["gap-115bae"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chat_host_preset_notes_a_cerebras_data_model' crates/ && cargo test -p roko-cli --lib chat_host_preset_notes_a_cerebras_data_model"
+++

## Problem

`roko config preset chat-host` picks the cheapest model of the ladder that roko calls over an API as the data model
(`[agent.data_llm]`). In Will's workspace that is `cerebras-gptoss`. Cerebras rate-limited the demo runs of
2026-10-15 (hermes-roko round v5, roko-demos: demo-hello took three attempts on "provider error: rate limited"),
and when a data-model call fails, roko withholds that tool output from the agent: it gets a notice instead
(`crates/roko-agent/src/safety/data_llm.rs::screen_result`, `ToolError::UntrustedContentWithheld`). The preset says
nothing about this trade-off.

## Why it matters

A person who accepts the preset's pick can find chat-started runs failing on withheld tool output, with no hint
that the data model is the cause or that two flags avoid it.

## Where

`crates/roko-cli/src/chat_host.rs::plan_chat_host` (the pick) and `crates/roko-cli/src/commands/tune.rs`
(`cmd_chat_host_preset`, the output).

## Current state

Open at `0cef16f64` (gap-115bae). Will decides the default (the lead put it to him); this item only adds the note.

## Plan

When the preset picks a data model on Cerebras (provider kind `cerebras_api`, or a base URL on `cerebras.ai`), add a
short note to its output and to `--json` (`note`): rate limits can hold back tool output, `--data-llm-model`
picks another data model, `--allow-without-data-llm` runs without one. Do not change the default.

## Done when

`chat_host_preset_notes_a_cerebras_data_model` passes: a pick on Cerebras carries the note, one elsewhere none.

## Notes

2026-10-16, roko-chathost (hermes-roko round v5): asked for by the round's lead; drafted but not applied, compiled
or tested, to keep usage free for the round's final dogfood. The draft is
`tmp/hermes-roko-v5/roko-chathost/tmp/next/patch_cerebras.py` in the main checkout (local, gitignored); apply it on
a branch from main with `python3 <it> <worktree root>`. It adds `note` to `ChatHostPlan` and `ChatHostPreset`
(`--json` gains `"note"`), `chat_host::rate_limit_note` (Cerebras = provider kind `cerebras_api` or a base URL on
`cerebras.ai`, judged through `resolve_model`), a `note: …` line in the text output, and the test
`chat_host::tests::chat_host_preset_notes_a_cerebras_data_model`. Wording: "`<model>` runs on Cerebras, whose rate
limits can hold back tool output: when a data-model call fails, the agent gets a notice instead of what the tool
returned. --data-llm-model <model> picks another data model; --allow-without-data-llm runs without one (local use)".
The Hermes plugin's `hermes roko init` always passes `--allow-without-data-llm`, so the note never reaches it there.
