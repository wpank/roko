+++
id = "gap-7ebc4f"
kind = "gap"
title = "roko-runtime's channel layer (platforms.rs, channel_binding.rs, adapters/) is outside the module tree and never compiled: park it behind a channels feature"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-runtime/channels"]
created = 2026-10-15
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "e5f84c149"
source = "status-quo-2026-10-15#decision-12"
anchors = ["crates/roko-runtime/src/lib.rs", "crates/roko-runtime/Cargo.toml", "crates/roko-runtime/src/platforms.rs::ChatBridge", "crates/roko-runtime/src/channel_binding.rs::ChannelBindingRouter", "crates/roko-runtime/src/adapters/mod.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-63055e", "gap-8d80d3", "spec-743a7e", "spec-b3ab28", "spec-b4ba72", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '^channels *=' crates/roko-runtime/Cargo.toml && grep -q 'feature = \"channels\"' crates/roko-runtime/src/lib.rs && grep -q 'mod channel_binding' crates/roko-runtime/src/lib.rs && cargo test -p roko-runtime --features channels channel_binding"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T03:07:13Z"
commit = "e5f84c149"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-17T02:10:53Z"
forced = false
evidence = "e5f84c149 parks the channel layer. roko-runtime's new `channels` feature (off by default: `dep:futures` plus `roko-core/channels`) gates `pub mod platforms`, `adapters` and `channel_binding` in crates/roko-runtime/src/lib.rs. roko-core's new `channels` feature gates `config::platforms` and `config::channels` in crates/roko-core/src/config/mod.rs; the layer imports both, and they were orphans too. The files compiled as they were, apart from rustfmt and two clippy fixes: `platform_id` returns `&'static str` in the six adapters (unnecessary_literal_bound), and PlatformCapabilities derives Default (derivable_impls). No adapter was deleted, and roko-cli and roko-serve don't forward the feature, since nothing there uses it. The literal verify passes: the three greps, then `cargo test -p roko-runtime --features channels channel_binding` with 4 passed. Also green on e5f84c149: `cargo test -p roko-runtime --features channels` (280 lib tests, 67 of them the layer's, plus the integration tests and doctests, with no rustc warnings), `cargo test -p roko-core --features channels --lib -- config::channels config::platforms` (14 passed), clippy `--no-deps -- -D warnings` on roko-core and roko-runtime with and without the feature, and `cargo +nightly fmt --all --check`. The default build is unchanged; with the feature, `cargo tree -p roko-runtime -e normal` gains futures and futures-executor (208 to 210 lines). CI's parked-feature step now runs `cargo check -p roko-runtime --features channels`. The `channels` row is in CLAUDE.md's and docs/v3/00-INDEX.md's park tables, and benchmarks/park/2026-10-17-200ced75c.json counts the 2,900 gated lines. Filed gap-092d24 (seven more channel-layer orphans and roko-serve's routes/platforms.rs), gap-d4da60 (snapshot_checkpoint.rs) and bug-2d3249 (the adapters' byte-120 preview panics on multibyte text)."
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
