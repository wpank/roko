+++
id = "bug-7567eb"
kind = "bug"
title = "Hermes tool-call parser executes <tool_call> blocks inside <think> reasoning"
status = "done"
triage = "verified"
severity = "p1"
size = "S"
goal = "hermes"
subsystem = ["roko-agent/translate"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::parse_calls", "crates/roko-agent/src/translate/hermes.rs::extract_text"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn parse_ignores_tool_call_inside_think' crates/roko-agent/src/translate/hermes.rs && cargo test -p roko-agent --lib translate::hermes::tests::parse_ignores_tool_call_inside_think"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:15Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:37:10Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`HermesXmlTranslator::parse_calls` (`crates/roko-agent/src/translate/hermes.rs:92`) executes tool calls that the model only drafted inside its reasoning. The module doc promises "`<think>...</think>` reasoning blocks are skipped" (`hermes.rs:34`). The parser has no reasoning handling at all: it scans the whole text with `text[search_from..].find("<tool_call>")` and parses every block it finds.

Repro (as a unit test on the translator):

```text
<think>
Maybe I should run <tool_call>{"name": "write_file", "arguments": {"path": "x.rs", "content": ""}}</tool_call>
... no, read it first.
</think>
<tool_call>{"name": "read_file", "arguments": {"path": "x.rs"}}</tool_call>
```

- Actual: two calls, `write_file` and then `read_file`, so the drafted `write_file` runs.
- Expected: one call, `read_file`.

A second case: if the reasoning block is unterminated (a truncated response), an unclosed `<tool_call>` inside it is parsed to the end of the text and executed. The unclosed-tag fallback (`hermes.rs:101-110`, tested by `parse_handles_unclosed_tool_call_tag`) is deliberate for normal text, but it should not apply inside reasoning.

The only reasoning test, `parse_skips_think_blocks` (`hermes.rs:440`), has no tool call inside the think block, so it passes by accident.

## Why it matters

- Goal `hermes`: this is the tool-call format for Hermes and Qwen3 models in the Nous/Hermes demo. `translator_for` (`crates/roko-agent/src/translate/capability.rs:171-184`) sends every model whose capability `tool_format` is `HermesJson` to this translator. `qwen3-32b` is one, per the test at `capability.rs:459`. Qwen3 and Hermes reasoning variants emit `<think>` blocks by default.
- Safety risk (p1): a destructive call the model considered and rejected (write, delete, shell) is executed anyway. This bypasses the model's own judgement, and the user never sees the call as a decision.
- Related items in the same file:
  - `bug-0a1729`: `repair_json` removes commas inside JSON strings;
  - `bug-b14145`: only the first call of a `tool_calls` wrapper is kept;
  - `spec-afe4aa`: the deep Hermes/Nous integration spec.

## Where

- `crates/roko-agent/src/translate/hermes.rs::HermesXmlTranslator::parse_calls` (:92-127): the scan loop to change.
- `hermes.rs::extract_text` (:161): builds the text from `BackendResponse::Text`, from `Json` `/message/content` or `/choices/0/message/content`, or from `StreamJson` content deltas. Reasoning sent in a separate JSON field (`reasoning_content`) is already excluded; only inline `<think>` text is the problem.
- `hermes.rs::parse_tool_call_body` (:200): parses one block body. No change needed.
- Tests module in `hermes.rs`: `parse_skips_think_blocks` (:440) and `parse_handles_unclosed_tool_call_tag` (:539) must keep passing.
- Existing helper: `crates/roko-cli/src/clean.rs::strip_thinking` (:139) and `remove_tag` (:147). They remove `<think>`/`<thinking>` blocks, and everything after an unterminated opener. `roko-agent` cannot depend on `roko-cli`, so the helper cannot be called from here as it is.
- Entry point: any API tool loop whose model resolves to `ToolFormat::HermesJson` (for example the OpenAI-compatible provider serving `qwen3-*` or Hermes models) calls `translator.parse_calls(&response)` each turn.

## Current state

- HEAD has no reasoning handling in `parse_calls`. There are no recent functional changes to `hermes.rs`: the last commits are cleanup (`8b4e3dfd3`, `59a1db0f4`), and the translator arrived in `72e0a76b8`.
- Unknown: whether any Hermes provider strips `<think>` before the translator sees the text. A grep of `crates/roko-agent/src/hermes/` and `provider/hermes.rs` found no `<tool_call>` or `parse_calls` handling, so assume not.

## Plan

1. Change the scan in `parse_calls` into one left-to-right pass over reasoning and tool-call tags, rather than stripping reasoning first. Stripping first would corrupt a legitimate call whose arguments contain the literal text `<think>`, for example `write_file` on a prompt template. At each step, find the earliest of `<think>`, `<thinking>`, `<reasoning>` and `<tool_call>` from `search_from`:
   - reasoning opener first: jump past its matching closer (`</think>`, `</thinking>`, `</reasoning>`). If there is no closer, stop scanning, because the rest of the text is unterminated reasoning, and parse nothing more;
   - `<tool_call>` first: keep the current logic, including the unclosed-tag fallback to end of text.
2. Keep the tag list in one `const` slice so the other translators can reuse it later. If you would rather share code with `clean.rs::strip_thinking`, move `remove_tag` into `roko-agent` (for example `translate/reasoning.rs`) and have `roko-cli/src/clean.rs` call it. That is optional and adds a second file to the change.
3. Add tests in the `hermes.rs` tests module:
   - `parse_ignores_tool_call_inside_think`: a call inside `<think>...</think>` and another after it. Assert exactly one call, the one after (this is the name the verify command expects);
   - `parse_ignores_unclosed_tool_call_inside_unterminated_think`: `<think>` with no closer that contains `<tool_call>{...}` with no closer. Assert zero calls;
   - `parse_keeps_think_text_inside_tool_call_arguments`: a call whose `content` argument contains `<think>` parses with the argument intact;
   - also cover `<thinking>` in one of these.
4. Update the module doc (`hermes.rs:34`) to list the reasoning tags that are skipped.

## Done when

- A `<tool_call>` inside a closed or unterminated reasoning block is never returned by `parse_calls`. Calls after a closed reasoning block still are.
- `parse_skips_think_blocks` and `parse_handles_unclosed_tool_call_tag` still pass.
- Verify: `grep -q 'fn parse_ignores_tool_call_inside_think' crates/roko-agent/src/translate/hermes.rs && cargo test -p roko-agent --lib translate::hermes::tests::parse_ignores_tool_call_inside_think`

## Notes

- Keep the change inside `parse_calls`. Do not change `repair_json` or `tool_call_from_value`: those are `bug-0a1729` and `bug-b14145`. All three touch `hermes.rs`, so run them one after another or expect a trivial merge.
- Other translators (`ReActTranslator`, Qwen XML) may have the same weakness. That is out of scope here; if you confirm it, file a separate item.
- Size S. Pure parser change with unit tests; no I/O or config.
- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  - `parse_calls` now makes one left-to-right pass over reasoning openers (`<think>`, `<thinking>`, `<reasoning>`,
    in `REASONING_TAGS`) and `<tool_call>`, taking whichever comes first (`next_opener`):
    - a reasoning block is skipped to its closer, and an unterminated one ends the scan;
    - a `<tool_call>` keeps the old handling, including the unclosed-tag fallback.
  - So reasoning text inside a call's arguments is untouched. The module doc lists the skipped tags.
  - Tests: `parse_ignores_tool_call_inside_think`, `parse_ignores_unclosed_tool_call_inside_unterminated_think`
    (which also covers `<thinking>`) and `parse_keeps_think_text_inside_tool_call_arguments`.

## Original notes


The module doc says "`<think>...</think>` reasoning blocks are skipped" (`translate/hermes.rs:34`), but `parse_calls` (`:92`) scans the whole response with `find("<tool_call>")` and has no reasoning-block handling.
The only think test (`parse_skips_think_blocks`, `:440`) has no tool call inside the think block. A call the model merely drafts while reasoning is executed, and an unclosed `<tool_call>` parses to end of text.
Fix: strip reasoning blocks (think/thinking/reasoning, including unterminated ones) before scanning; add tests for a call inside `<think>` (must not execute) and after it (must execute).
