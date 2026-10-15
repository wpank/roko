+++
id = "bug-4e336b"
kind = "bug"
title = "roko init writes a model ladder whose lower rungs name models the workspace cannot run, so every task starts on the top rung; init --model is ignored"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/init"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-13
source = "hermes-plugin-roko docs walkthrough 2026-10-13 (roko 5de43f9ab)"
anchors = ["crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn init_writes_a_runnable_ladder_and_honours_model' crates/roko-cli/ && cargo test -p roko-cli init_writes_a_runnable_ladder_and_honours_model"
+++

## Problem

`roko init` writes a `[routing.ladder]` whose `cheap`, `mid` and `strong` rungs name `gpt-oss-120b`, `glm-4.7`
and `gpt-5.4-mini`, none of which the workspace's `[models]` defines (it defines only `claude-sonnet-4-6` on
`claude_cli`). The ladder keeps only runnable rungs, so every task starts on `top` (seen 2026-10-13, roko
`5de43f9ab`, a fresh Rust crate on Will's machine). `roko init --model glm-5-1` changes nothing either: the
written `default_model` stays `claude-sonnet-4-6`.

## Why it matters

A new workspace routes everything to its most expensive rung, and with only the Claude CLI configured, a chat
host's runs cannot run at all (see the related up-front refusal item).

## Where

- `crates/roko-cli/src/commands/init*.rs` (the template roko.toml and the `--model` flag).
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder::start`.

## Plan

1. Write ladder rungs only for models the new workspace can run, or none, and honour `roko init --model`.
2. Test: `roko init --model X` writes `default_model = "X"`, and every rung it writes resolves to a configured
   model.

## Done when

`init_writes_a_runnable_ladder_and_honours_model` passes (the `[[verify]]` command).

2026-10-15 (status-quo assessment, finding 9): still open at b42578bab. No commit since 2026-10-13 touches `graph_task_dispatch/verification.rs` or `plan_generate/`, and no other item links here. Seen again in the 10-14 tally runs: all 28 ladder records are rung `top`, reason `start`, because the workspace's cheap, mid and strong rungs (gpt-oss-120b, glm-4.7, gpt-5.4-mini) had no usable provider (`tmp/status-quo/2026-10-15/programme/E2a-matrix-retag-AU-EX-IS-QA-RC.md:168`, row EX7). The 49-run sweep found no climb anywhere (71 ladder records, all step 0). The cheap-model ladder claim (gap-f30b8e, funded by decision 5) can't be shown on a workspace `roko init` made until this is fixed.
