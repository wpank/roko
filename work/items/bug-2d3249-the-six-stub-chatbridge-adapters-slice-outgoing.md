+++
id = "bug-2d3249"
kind = "bug"
title = "The six stub ChatBridge adapters slice outgoing text at byte 120 for their debug preview, which panics when that byte falls inside a multibyte character"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-runtime/channels"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "gap-7ebc4f"
discovered_from = "gap-7ebc4f (worker, 2026-10-17): reading the adapters while parking them"
anchors = ["crates/roko-runtime/src/adapters/whatsapp.rs::WhatsAppBridge", "crates/roko-runtime/src/adapters/discord.rs::DiscordBridge", "crates/roko-runtime/src/adapters/telegram.rs::TelegramBridge", "crates/roko-runtime/src/adapters/mod.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-7ebc4f", "gap-63055e", "spec-b3ab28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn log_preview_keeps_whole_characters' crates/roko-runtime/src/adapters/ && cargo test -p roko-runtime --features channels log_preview_keeps_whole_characters"
+++

## Problem

Each stub `ChatBridge` adapter under `crates/roko-runtime/src/adapters/` logs a preview of the outgoing text by
slicing it at byte 120: `&text[..text.len().min(120)]` (`&body[..]` in `send_rich`). Slicing a `str` at a byte that
is not a character boundary panics. Take a 119-byte ASCII prefix followed by `é` (bytes 119-120), or any
non-Latin text longer than 120 bytes: `send_message` panics with "byte index 120 is not a char boundary" instead of
returning a `MessageId`.

- `whatsapp.rs` takes the slice before the `tracing::debug!` call (`let preview = &text[..text.len().min(120)];`
  in both `send_message` and `send_rich`), so it always panics on such input.
- `discord.rs`, `slack.rs`, `matrix.rs`, `mattermost.rs` and `telegram.rs` slice inside the `debug!` arguments,
  so they panic only when DEBUG logging is enabled for that target.

## Why it matters

The adapters are parked (`--features channels`, gap-7ebc4f), so no default build reaches them. But anyone who
revives the channel layer (the held gap-63055e or spec-b3ab28) inherits a crash on ordinary non-ASCII chat text,
and the stubs' own tests use only ASCII. Goal `tooling`.

## Where

- `crates/roko-runtime/src/adapters/whatsapp.rs::WhatsAppBridge` (`send_message`, `send_rich`)
- The same two methods in `discord.rs`, `slack.rs`, `matrix.rs`, `mattermost.rs` and `telegram.rs`
- `crates/roko-runtime/src/adapters/mod.rs`: a natural home for one shared helper

## Current state

The adapters compile and their tests pass under `cargo test -p roko-runtime --features channels adapters`, and every
test message is ASCII.

## Plan

1. Add one helper in `adapters/mod.rs`, for example `log_preview(text: &str) -> &str`, that cuts at the last
   character boundary at or before byte 120 (`str::floor_char_boundary`, or a `char_indices` loop).
2. Use it in all twelve send methods.
3. Add the test `log_preview_keeps_whole_characters`: the helper on `"a".repeat(119) + "é"`, plus a
   `WhatsAppBridge::send_message` call with that text, which must return `Ok`.

## Done when

The `[[verify]]` passes: `log_preview_keeps_whole_characters` exists and passes with `--features channels`.

## Notes

Change nothing else in the stubs. Their behaviour, capabilities and message ids stay as they are.
