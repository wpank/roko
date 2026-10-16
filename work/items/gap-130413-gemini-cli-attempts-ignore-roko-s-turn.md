+++
id = "gap-130413"
kind = "gap"
title = "Gemini CLI attempts ignore roko's turn cap: write it as the CLI's model.maxSessionTurns setting (gap-ccbf26 step 2)"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/gemini-cli"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "gap-ccbf26 step 2"
anchors = ["crates/roko-agent/src/provider/gemini_cli.rs::GeminiCliAdapter", "crates/roko-agent/src/provider/error_classify.rs::detect_turn_cap"]
links = { depends_on = [], blocks = [], related = ["gap-ccbf26"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_gemini_cli_attempt_runs_under_roko_s_turn_cap' crates/roko-agent/src && cargo test -p roko-agent --lib a_gemini_cli_attempt_runs_under_roko_s_turn_cap"
+++

## Problem

roko's turn cap is advisory for a Gemini CLI attempt (`TurnCapEnforcement::Advisory`). The CLI has no turn flag, so
only the attempt timeout ends a long run (gap-ccbf26). Gemini CLI 0.41.2 does have a `model.maxSessionTurns` setting
(default -1, unlimited). With it set, its non-interactive loop stops after that many turns: it prints "Reached max
session turns for this session" and exits non-zero. roko never sets it.

## Why it matters

A capped run on a Gemini CLI rung is bounded only by its timeout. With the turn cap held natively, a budget-derived
cap (gap-ccbf26 step 3) has a lever to pull.

## Where

- `crates/roko-agent/src/provider/gemini_cli.rs::GeminiCliAdapter`: builds the run (`gemini -m <slug> -p`).
- `crates/roko-agent/src/provider/error_classify.rs::detect_turn_cap`: what counts as a turn-cap stop.

## Plan

1. When the attempt has a turn cap, write a run-scoped system settings file with `model.maxSessionTurns` set to it,
   and point the CLI at it with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`. System settings override user and project ones.
   Merge in the system settings that would otherwise apply, and keep a lower limit an admin set.
2. Count Gemini's "Reached max session turns" stop as a turn-cap hit, as Claude's `error_max_turns` is, so the task
   retries with a raised cap instead of failing over.
3. Report `TurnCapEnforcement::Native` for the Gemini CLI.

## Done when

`a_gemini_cli_attempt_runs_under_roko_s_turn_cap` passes (the `[[verify]]` command).

## Notes

Step 2 of gap-ccbf26, split out at the lead's request. Checked against Gemini CLI 0.41.2's bundle: the
`maxSessionTurns` setting, `GEMINI_CLI_SYSTEM_SETTINGS_PATH`, and the turn-limit stop.
