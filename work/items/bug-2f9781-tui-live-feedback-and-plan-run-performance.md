+++
id = "bug-2f9781"
kind = "bug"
title = "TUI Live Feedback and Plan Run Performance Gaps"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/108-tui-live-feedback-gaps.md#108 — TUI Live Feedback and Plan Run Performance Gaps"
discovered_from = "audit:tmp/backlog/archive/108-tui-live-feedback-gaps.md#108 — TUI Live Feedback and Plan Run Performance Gaps"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:994", "crates/roko-cli/src/runner/graph_tui_bridge.rs", "crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui"]
links = { depends_on = [], blocks = [], related = ["bug-45c355", "bug-70fd41"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-28
evidence = "obsolete umbrella: its partial 5/6 status and acceptance steps target the deleted Runner-v2 inline TUI (runner/event_loop.rs, --engine runner-v2). On the Graph path the inline TUI auto-launches in an interactive terminal (crates/roko-cli/src/graph_execution/plan_runner.rs:994) and task_started is emitted before execution (runner/graph_tui_bridge.rs:115); the remaining live-feedback gap is tracked by bug-45c355 (agent_spawned only after dispatch) and bug-70fd41 (model not forwarded)"
+++
[partial] 00-INDEX historical claim: PR #73, 5/6 sub-items wired — makes plan execution feel broken even when it is working; zero user feedback during 2.5+ minute API calls. Running `roko plan run` with the inline TUI (`--approval`) or watching from the standalone `roko dashboard` should give the…

Imported without verification from:
- `tmp/backlog/archive/108-tui-live-feedback-gaps.md#108 — TUI Live Feedback and Plan Run Performance Gaps`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/tui/app.rs`.

How to verify: Check: In an interactive terminal, `roko plan run plans/` (without `--approval`) shows the inline TUI automatically.; During a long API call (>15 seconds), the TUI shows "agent running (Xs elapsed)" even when no token events have arrived.; The… [evidence: 00-INDEX historical claim: PR #73, 5/6 sub-items wired; 00-STATUS-SUMMARY 2. Partial / : 5/6 sub-items wired (PR #73); 1 sub-item remaining]

Verified 2026-09-28: superseded - Runner-v2 targets deleted; Graph remainder tracked in bug-45c355 / bug-70fd41.
