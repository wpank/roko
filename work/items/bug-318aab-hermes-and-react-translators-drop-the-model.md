+++
id = "bug-318aab"
kind = "bug"
title = "Hermes and ReAct translators drop the model's own tool-call turn from the history"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/translate"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-d0b8b8"
anchors = ["crates/roko-agent/src/translate/hermes.rs", "crates/roko-agent/src/translate/react.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-d0b8b8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib render_assistant_message_keeps_the_tool_call"
+++

## Problem

`HermesXmlTranslator::render_assistant_message` returns None, and so does ReActTranslator's. The tool loop therefore never records the model's own `<tool_call>` turn in the history, and the next request carries only the `<tool_response>`.

## Plan

Render the assistant turn with its tool-call text. Add a test named `render_assistant_message_keeps_the_tool_call_*`.

## Done when

- `cargo test -p roko-agent --lib render_assistant_message_keeps_the_tool_call` passes.

## Notes

- Reported on 2026-10-01 by wk-tiers, working on bug-d0b8b8, during the evening close-out round.
