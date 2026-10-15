+++
id = "gap-7ebc4f"
kind = "gap"
title = "roko-runtime's channel layer (platforms.rs, channel_binding.rs, adapters/) is outside the module tree and never compiled: park it behind a channels feature"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-runtime/channels"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-12"
anchors = ["crates/roko-runtime/src/lib.rs", "crates/roko-runtime/Cargo.toml", "crates/roko-runtime/src/platforms.rs::ChatBridge", "crates/roko-runtime/src/channel_binding.rs::ChannelBindingRouter", "crates/roko-runtime/src/adapters/mod.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-63055e", "gap-8d80d3", "spec-743a7e", "spec-b3ab28", "spec-b4ba72", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '^channels *=' crates/roko-runtime/Cargo.toml && grep -q 'feature = \"channels\"' crates/roko-runtime/src/lib.rs && grep -q 'mod channel_binding' crates/roko-runtime/src/lib.rs && cargo test -p roko-runtime --features channels channel_binding"
+++

## Problem

roko-runtime carries a channel layer that is not in its module tree at all: `crates/roko-runtime/src/platforms.rs`
(792 lines, `ChatBridge`, `PlatformConfig`), `crates/roko-runtime/src/channel_binding.rs` (227 lines,
`ChannelBindingRouter`) and `crates/roko-runtime/src/adapters/` (`mod.rs`, discord, matrix, mattermost, slack, telegram,
whatsapp). `crates/roko-runtime/src/lib.rs` declares none of them (`grep -n 'mod platforms\|mod channel_binding\|mod
adapters' crates/roko-runtime/src/lib.rs` is empty), so they are never compiled, never tested, and rot silently; nothing
outside roko-runtime names them. They were last touched by `9c6ec420c`.

## Why it matters

Will's decision 12 (2026-10-15, `tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`): park the orphaned channel-layer
files behind a feature, as decision 9201 parked the other unused subsystems; the channel items stay held (dec-e70592:
gap-63055e, gap-8d80d3, spec-743a7e, spec-b3ab28, spec-b4ba72). A parked feature compiles on demand and shows up in the
park table; orphaned files do neither. Goal `tooling`.

## Where

- `crates/roko-runtime/src/lib.rs` (module declarations), `crates/roko-runtime/Cargo.toml` `[features]` (today only
  `cognitive-clock`).
- The three paths above; `channel_binding.rs` uses `roko_core::config::channels::ChannelConfig`, `crate::pulse_bus` and
  `crate::reactive_agent`.
- CLAUDE.md's "Parked (off the default build)" table and `benchmarks/park/` (line counts).

## Current state

Not compiled since they left the module tree; they may not build against today's roko-core and roko-runtime.

## Plan

1. Add `channels = []` (plus any optional dependencies the adapters need, as `dep:` entries) to roko-runtime, off by
   default, and declare `#[cfg(feature = "channels")] pub mod platforms; pub mod channel_binding; pub mod adapters;`.
2. Make them compile and their own tests pass under the feature, changing as little as possible; if an adapter cannot be
   revived cheaply, delete that adapter and say so in the commit.
3. Forward the feature from roko-cli and roko-serve only if something there needs it (today nothing does).
4. Add a row to CLAUDE.md's park table and a line count under `benchmarks/park/`.

## Done when

The `[[verify]]` command passes: the feature exists, gates the modules, and `cargo test -p roko-runtime --features
channels channel_binding` passes. A default build is unchanged.

## Notes

Do not wire the layer into serve or the runtime; this only parks it. The held items stay held.
