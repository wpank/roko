+++
id = "gap-085abb"
kind = "gap"
title = "Should a trigger whose source is a chat host get the same outbound stage floor as a chat-hosted run?"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "M"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-18 follow-up reports 2026-10-04 (gap-1a4563, work/gap-1a4563)"
discovered_from = "gap-1a4563 (open; its fix covers chat-hosted and direct plan submission, not triggers)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::GraphPlanRunParams", "crates/roko-serve/src/runtime.rs::RunOrigin"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chat_sourced_trigger_run_gets_the_stage_floor' crates/ && cargo test -p roko-cli chat_sourced_trigger_run_gets_the_stage_floor"
+++

## Problem

Chat-host plans now get an outbound floor of `stage` and CLI agents refuse a staged contract
(`gap-1a4563`'s fix, on `work/gap-1a4563`, not yet merged), but runs started by triggers still
follow decision 9107's domain default (`stage` only in the `ops` domain) with no special floor
of their own.

`outbound_floor(origin: &RunOrigin) -> Option<OutboundPolicy>`
(`crates/roko-cli/src/graph_execution/plan_runner.rs:941-952`) returns `Some(Stage)` only
`if origin.is_chat()`, else `None`. `RunOrigin` (`crates/roko-serve/src/runtime.rs:172-183`) has
exactly three variants: `Cli`, `Http` (default) and `Mcp { client }` (a chat host over `POST
/mcp`; `is_chat()` is `matches!(self, Self::Mcp { .. })`). Confirmed: neither
`crates/roko-serve/src/trigger_runtime.rs` nor `crates/roko-cli/src/commands/trigger.rs`
references `RunOrigin` or `outbound_floor` at all — a trigger-started run gets none of this
mechanism, so `outbound_floor` is never `Some(Stage)` for it regardless of what fired it;
whatever the plan's `[meta] outbound` or the task's domain says is all it gets (decision 9107's
default, `stage` in `ops` only).

There's a deeper wrinkle even if the answer is "yes, cover it": `TriggerSource`
(`crates/roko-core/src/trigger.rs:492`) has no variant that identifies "this came from a chat
host" — its seven variants are `Cron`, `Webhook`, `FileWatch`, `Bus`, `ChainEvent`, `Manual`,
`SignalPattern`. A chat-platform-originated trigger today is just a `Webhook` like any other;
nothing distinguishes it from a CI webhook or a generic integration's callback.

## Why it matters

Goal: release/safety, same goal as `gap-1a4563` (9131). The whole point of the chat-host floor
is that untrusted, free-text-originated requests get held for approval before they act on the
outside world (9117: "a chat host's run... request text is untrusted data"). A trigger fired by
a chat platform (a Slack slash command, a Discord bot webhook) carries the same kind of
untrusted, externally-originated request text, but today gets only the domain default — `stage`
only if its task happens to be in the `ops` domain, nothing otherwise — which could let a
chat-sourced trigger run with less scrutiny than the exact same request would get if a user
typed it into a chat host directly against `roko run`.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::outbound_floor` (the floor decision).
- `crates/roko-serve/src/runtime.rs::RunOrigin` (no trigger-sourced variant).
- `crates/roko-core/src/trigger.rs::TriggerSource` (no chat-sourced variant to key a decision
  on, even if one is wanted).
- `crates/roko-serve/src/trigger_runtime.rs`, `crates/roko-cli/src/commands/trigger.rs` (where
  a trigger starts a run; neither touches `RunOrigin`/`outbound_floor` today).

## Plan (decision needed)

- **Option A — give a trigger fired from a chat-platform source the same `stage` floor.** This
  needs a way to mark that a `Webhook` (or other) trigger's source is chat-like — either a new
  `TriggerSource` field/variant, or a convention (e.g. a configured webhook URL pattern or a
  header check) — plus wiring `outbound_floor` (or an equivalent call at the trigger-run-start
  site) to apply `Stage` when that mark is present.
- **Option B — leave triggers on the domain default.** Decide this is acceptable because a
  trigger's registration is itself an operator action (someone configured the webhook/cron/etc.
  deliberately), unlike a chat host's free-text request, which is the untrusted part 9117 is
  really about — document this distinction explicitly rather than leaving it implicit.
- **Option C — give every trigger-started run the `stage` floor regardless of source**, treating
  "externally triggered, not a direct CLI/API call a human is watching" as the risk signal
  rather than "chat-sourced" specifically.

## Done when

Will picks an option; the chosen behavior is implemented (if A or C) or explicitly documented
as intentional (if B), with a regression test covering whichever trigger-run path the decision
settles on.

## Notes

- 2026-10-04 (wave-18 follow-up, gap-1a4563, work/gap-1a4563 not yet merged): confirmed at main
  HEAD `c82aa1837` (`RunOrigin`, `TriggerSource` and both trigger-runtime files are unchanged by
  that branch's diff, so verifiable on `main` directly; `plan_runner.rs::outbound_floor` is the
  one branch-only piece, confirmed via `git show`). Filed as `kind = "gap"` with the decision
  embedded in the body, per the instruction.
