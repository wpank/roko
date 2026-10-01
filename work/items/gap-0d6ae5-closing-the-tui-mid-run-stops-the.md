+++
id = "gap-0d6ae5"
kind = "gap"
title = "Closing the TUI mid-run stops the plan run; there is no detach mode"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-cli/graph_execution", "roko-cli/tui"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::pending_interrupt", "crates/roko-cli/src/graph_execution/plan_runner.rs::PlanRunInterrupt", "crates/roko-cli/src/serve_client.rs::follow_run_tui"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'stopping the plan run' crates/roko-cli/src/graph_execution/plan_runner.rs"
+++

Quitting the TUI while plans are still running is treated as an interrupt (`PlanRunInterrupt::Interrupt`, exit 130). Before the interrupt work, the run continued headless with stderr redirected and no terminal output. Neither suits an operator who closes the dashboard to look at something else.

Fix: offer a detach choice that keeps the run going and falls back to inline progress on the terminal (or to a hub the operator can reattach to).

Re-verified 2026-09-29 at d9e79e9d8: when a live roko-serve owns the workspace, roko plan run forwards the run to it (commands/plan.rs:565-575, 08a1fd272). Closing the follow TUI there does not cancel the run (serve_client.rs:723-741), so it acts as an implicit detach. What remains: local runs with no server still turn a TUI close into PlanRunInterrupt::Interrupt (plan_runner.rs:472-482), and neither path offers an explicit detach choice or falls back to inline terminal progress.

## Notes

2026-10-01 (wk-runstate): blocked. Two things stop this: an explicit detach choice is TUI work in another packet's files, and it needs a UX decision. Still true at BASE: a local run turns a TUI close into `PlanRunInterrupt::Interrupt` (`plan_runner.rs::pending_interrupt`), and the test `closing_the_tui_mid_run_interrupts_the_run` pins that behaviour. The choice belongs in the TUI's quit confirmation (`ModalState::Quit`: `tui/app/actions.rs`, `tui/input.rs::handle_confirm_key`, `tui/modals/mod.rs`, `tui/widgets/status_bar.rs`). Proposed next step: give the quit modal a `d` (detach) answer that sets a flag the runner passes in, for example `App::with_detach_flag(Arc<AtomicBool>)`. With the flag set, `pending_interrupt` keeps the run going: it drops the `TuiSession`, which points stderr back at the terminal, and turns on the inline progress sink. `InlineProgressTelemetrySink::show_progress` would become a shared `AtomicBool` for that. Ctrl-C would still stop the run, and `roko dashboard` could reattach to it through `.roko/events.jsonl`. Will should first decide whether quitting keeps stopping the run, with detach as the opt-in answer as proposed, or whether detach becomes the default. Also: the [[verify]] cannot pass as written, because "the conductor is stopping the plan run" (`plan_runner.rs:1247`) also matches `stopping the plan run`. Narrow it to the TUI-close message when this is done.
