+++
id = "bug-318aab"
kind = "bug"
title = "Hermes and ReAct translators drop the model's own tool-call turn from the history"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-agent/translate"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-d0b8b8"
anchors = ["crates/roko-agent/src/translate/hermes.rs", "crates/roko-agent/src/translate/react.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-d0b8b8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib render_assistant_message_keeps_the_tool_call"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:09Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:44Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`HermesXmlTranslator::render_assistant_message` returns None, and so does ReActTranslator's. The tool loop therefore never records the model's own `<tool_call>` turn in the history, and the next request carries only the `<tool_response>`.

## Plan

Render the assistant turn with its tool-call text. Add a test named `render_assistant_message_keeps_the_tool_call_*`.

## Done when

- `cargo test -p roko-agent --lib render_assistant_message_keeps_the_tool_call` passes.

## Notes

- Reported on 2026-10-01 by wk-tiers, working on bug-d0b8b8, during the evening close-out round.
- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  - `HermesXmlTranslator::render_assistant_message` returns the model's turn as
    `{"role": "assistant", "content": <its text>}`, `<tool_call>` blocks included. The text comes from the same
    `extract_text` the parser uses. `ReActTranslator` does the same for a `Text` response, `Action:` lines
    included. An empty turn adds nothing.
  - Tests: `render_assistant_message_keeps_the_tool_call_hermes` and `render_assistant_message_keeps_the_tool_call_react`
    (they replace `render_assistant_message_returns_none`). `hermes_json_profile_uses_hermes_translator` now checks
    that the second request carries the call turn before its `<tool_response>`.
