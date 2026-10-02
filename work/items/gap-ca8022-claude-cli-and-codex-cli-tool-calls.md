+++
id = "gap-ca8022"
kind = "gap"
title = "Claude CLI and Codex CLI tool calls leave no safety provenance"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli", "roko-agent"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "ac3cb2254"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ff95f5"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-cli/src/safety_provenance.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-ff95f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib cli_attempts_record_provenance"
+++

## Problem

CLI providers (Claude CLI, Codex CLI) run their tools outside ToolDispatcher, so gap-ff95f5's provenance sink sees none of their tool calls. That item allows at best one record per dispatch turn for them.

## Plan

Record one provenance record per CLI dispatch turn, from the live-output tap's tool events. Add a test named `cli_attempts_record_provenance`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-tamper, working on gap-ff95f5, during the overnight close-out round.
- 2026-10-02 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check.
  - After a Claude CLI or Codex CLI attempt settles, `emit_feedback` (`graph_task_dispatch/feedback.rs`,
    `record_cli_turn_provenance`) drains the attempt's live-output tap (`LiveToolCalls`, bug-264c41) and records one
    `ProvenanceOutcome` with its run's sink.
  - The record carries no intent (the CLI ran its tools before roko saw them), `call_id` `cli-turn`, tool
    `cli:<provider>`, the dispatch's success as its verdict, and a keyed digest of the observed calls (id, tool,
    outcome) as its argument digest. A turn that made tool calls carries external taint.
  - The plan run hands its `ProvenanceSinks` to `GraphFeedbackContext::provenance_sinks`. A failed write is logged,
    since the tools have already run.
  - Test: `cli_attempts_record_provenance` (roko-cli lib). Other CLI kinds (Cursor, Gemini CLI) are not covered.
