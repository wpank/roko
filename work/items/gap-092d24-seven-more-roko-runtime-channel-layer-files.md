+++
id = "gap-092d24"
kind = "gap"
title = "Seven more roko-runtime channel-layer files and roko-serve's routes/platforms.rs are outside the module tree and never compiled: park them under channels or delete them"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-runtime/channels"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "gap-7ebc4f"
discovered_from = "gap-7ebc4f (worker, 2026-10-17): parking the channel layer; 9c6ec420c lists these as modules not yet declared in any lib.rs"
anchors = ["crates/roko-runtime/src/lib.rs", "crates/roko-runtime/src/delivery.rs::DeliveryStateMachine", "crates/roko-runtime/src/message_formatter.rs::MessageFormatter", "crates/roko-runtime/src/command_dispatcher.rs::CommandDispatcher", "crates/roko-runtime/src/notification_batcher.rs::NotificationBatcher", "crates/roko-runtime/src/message_audit.rs::MessageAuditLog", "crates/roko-runtime/src/webhook_dead_letter.rs::DeadLetterWriter", "crates/roko-runtime/src/webhook_metrics.rs::WebhookMetrics", "crates/roko-serve/src/routes/platforms.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-7ebc4f", "gap-63055e", "gap-8d80d3", "spec-743a7e", "spec-b3ab28", "spec-b4ba72", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for m in command_dispatcher delivery message_audit message_formatter notification_batcher webhook_dead_letter webhook_metrics; do test ! -e crates/roko-runtime/src/$m.rs || grep -q \"mod $m;\" crates/roko-runtime/src/lib.rs || exit 1; done && ( test ! -e crates/roko-serve/src/routes/platforms.rs || grep -q \"mod platforms;\" crates/roko-serve/src/routes/mod.rs ) && cargo check -p roko-runtime --features channels"
+++

## Problem

Commit `9c6ec420c` (2026-09-25) added the chat-platform programme's files (#409-#430) without declaring them in
any module tree. gap-7ebc4f parked the three that decision 12 named (`platforms.rs`, `channel_binding.rs`,
`adapters/`) behind roko-runtime's `channels` feature. Seven more channel-layer files in roko-runtime, and one route
in roko-serve, are still outside every module tree, so no build compiles or tests them:

| File | Lines | What |
|---|---|---|
| `crates/roko-runtime/src/command_dispatcher.rs` | 346 | `CommandDispatcher`: roko slash commands parsed from chat messages (#429) |
| `crates/roko-runtime/src/delivery.rs` | 533 | `DeliveryStateMachine`: outbound message retries and dead-lettering (#415) |
| `crates/roko-runtime/src/message_audit.rs` | 271 | `.roko/audit/messages.jsonl`, an audit trail of `ChatBridge` traffic (#416) |
| `crates/roko-runtime/src/message_formatter.rs` | 249 | Per-platform formatters for `platforms::RichMessage` (#430) |
| `crates/roko-runtime/src/notification_batcher.rs` | 348 | `NotificationBatcher`: time-window batching of notifications (#417) |
| `crates/roko-runtime/src/webhook_dead_letter.rs` | 290 | `.roko/webhooks/dead-letter.jsonl` (#418) |
| `crates/roko-runtime/src/webhook_metrics.rs` | 147 | `WebhookMetrics` delivery counters (#420) |
| `crates/roko-serve/src/routes/platforms.rs` | 109 | `GET /api/platforms` and `GET /api/platforms/{id}` (#414) |

See it: `grep -n 'mod delivery\|mod message_formatter' crates/roko-runtime/src/lib.rs` prints nothing, and
`crates/roko-serve/src/routes/mod.rs` declares no `platforms` module.

## Why it matters

Decision 12 (2026-10-15) parked the orphaned channel layer so that it compiles on demand and shows up in the park
table. These files are the rest of that layer and keep rotting where nobody sees them. Goal `tooling`. Related:
gap-7ebc4f (the `channels` feature) and the held channel items (dec-e70592): gap-63055e, gap-8d80d3, spec-743a7e,
spec-b3ab28, spec-b4ba72.

## Where

- `crates/roko-runtime/src/lib.rs`: the module list. `#[cfg(feature = "channels")]` already gates `adapters`,
  `channel_binding` and `platforms`.
- `crates/roko-runtime/Cargo.toml`: `channels = ["dep:futures", "roko-core/channels"]`.
- The seven roko-runtime files above. `message_formatter.rs` imports `crate::platforms::RichMessage`, so it can only
  build under `channels`; the others use std, serde and tokio.
- `crates/roko-serve/src/routes/platforms.rs` names `roko_runtime::PlatformSnapshot` (roko-runtime has no such
  root re-export; the type is `roko_runtime::platforms::PlatformSnapshot`) and `AppState::platform_registry` (no
  such field).

## Current state

None of these files has been compiled since `9c6ec420c`, so they may not build against today's crates. The other
orphan from that commit, `snapshot_checkpoint.rs`, is StateHub code rather than channel code and has its own item.

## Plan

1. For each roko-runtime file, either declare it under `#[cfg(feature = "channels")]` in `lib.rs` and make it
   compile and pass its own tests with `--features channels`, or delete it if it can't be revived cheaply. Say
   which in the commit.
2. For `routes/platforms.rs` there's a design choice. Parking it needs a roko-serve `channels` feature that
   forwards `roko-runtime/channels`, plus an `AppState` registry field gated the same way. Deleting it is cheaper,
   and the held gap-63055e can rewrite the route when it is revived. Pick one and record it.
3. Update the `channels` line counts under `benchmarks/park/`, and CLAUDE.md's Parked row if what the feature
   brings back changes.

## Done when

The `[[verify]]` passes: each listed roko-runtime file is gone or declared in `lib.rs`, the serve route is gone or
declared in `routes/mod.rs`, and `cargo check -p roko-runtime --features channels` passes. A default build is
unchanged.

## Notes

Don't wire the layer into serve or the runtime. This item only parks or removes files, and the held items stay
held.
