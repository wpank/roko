+++
id = "bug-2fdfbd"
kind = "bug"
title = "roko acp may fail to parse spec embedded-resource and resource_link prompt blocks"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-426c9d"
anchors = ["crates/roko-acp/src/types.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-426c9d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib prompt_resource_blocks_parse"
+++

## Problem

Inbound prompt `resource` blocks deserialize into a `ResourceRef` that expects `{type:"file",uri}`. The spec's EmbeddedResource is `{uri, text|blob, mimeType}`, and `resource_link` blocks are not handled at all, so spec clients' embedded context probably fails to parse. Unverified.

## Plan

Accept the spec's EmbeddedResource and resource_link shapes, and test them against the vendored schema. Add a test named `prompt_resource_blocks_parse_*`.

## Done when

- `cargo test -p roko-acp --lib prompt_resource_blocks_parse` passes.

## Notes

- Reported on 2026-10-01 by wk-specq, working on bug-426c9d, during the evening close-out round.
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  Confirmed: an embedded resource failed to parse, which made the whole `session/prompt` invalid, even though roko
  advertises `embeddedContext`. A `resource_link` became `Unknown`, which `unsupported_prompt_content` refuses,
  though agents must accept links. `ResourceRef` now also has `Text` and `Blob` (manual serde; roko's
  `{"type":"file"}` form still parses), and `ContentBlock::ResourceLink` exists. File links and embedded text feed the
  prompt context on both context paths; links also appear in the prompt text. Tests: `prompt_resource_blocks_parse_*`
  (the samples are validated against the vendored schema).
