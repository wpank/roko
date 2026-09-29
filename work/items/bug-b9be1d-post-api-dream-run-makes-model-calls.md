+++
id = "bug-b9be1d"
kind = "bug"
title = "POST /api/dream/run makes model calls whenever providers are configured"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-serve/dream", "roko-dreams"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/dream.rs:63", "crates/roko-dreams/src/runner.rs::build_agent", "crates/roko-dreams/src/runner.rs::build_dream_review_dispatcher_with_model_caller"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn quick_dream_run_makes_no_model_calls' crates/roko-serve/ && cargo test -p roko-serve routes::dream::tests::quick_dream_run_makes_no_model_calls"
+++

The route builds a `DreamLoopConfig` with a placeholder agent command (`routes/dream.rs:64-69`), but `build_agent` (`roko-dreams/src/runner.rs:92`) ignores the command whenever the workspace has `[providers]` or `[models]` and dispatches review-agent calls with `agent.default_model` (`:97-105`).
A dashboard "run dream" click therefore spends model tokens with no budget, confirmation or cost accounting.
Fix: make model use explicit (e.g. `mode = quick|full`, quick makes no model calls), cap and account the spend, and report it in the response.
