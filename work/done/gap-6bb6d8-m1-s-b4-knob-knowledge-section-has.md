+++
id = "gap-6bb6d8"
kind = "gap"
title = "M1's B4 knob knowledge_section has no reader: a move on it changes nothing"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/dispatch", "roko-core/config"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "5783f69d0"
source = "status-quo-2026-10-15#finding-13"
anchors = ["crates/roko-core/src/config/harness_params.rs::HarnessParams", "crates/roko-cli/src/dispatch/prompt_builder.rs::withholds", "crates/roko-cli/src/graph_task_dispatch/verification.rs::error_pattern_limit", "crates/roko-learn/src/homeostasis/policy.rs::SafetyBox"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn knowledge_section_knob_off_omits_the_knowledge_section' crates/roko-cli/ && cargo test -p roko-cli knowledge_section_knob_off_omits_the_knowledge_section"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T20:18:29Z"
commit = "5783f69d0"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:20Z"
forced = false
evidence = "5783f69d0: each Graph attempt's DispatchContext and PromptContext carry knowledge_section, the knob of the θ the attempt runs (AttemptContext::knowledge_section; on without an M1 sink, as θ₀). With it off, PromptAssembler::assemble leaves the `knowledge` source section out (PromptContext::harness_withholds, beside the L-know arm's withholds): its entries are logged with the new excluded_reason harness_policy (ExcludedReason::HarnessPolicy), PromptDiagnostics::knowledge_section_off is set, and decision_log marks the knowledge row source = control and no L-know opportunity (reason knowledge_section_off), so the census credits neither L-know arm. Verify `cargo test -p roko-cli knowledge_section_knob_off_omits_the_knowledge_section` passed on the committed tree (1 passed; all 76 roko-cli test binaries built and ran clean); the dispatch::prompt_builder, graph_task_dispatch::decision_log and graph_task_dispatch::attempt lib tests pass (97); cargo clippy -p roko-learn -p roko-cli --no-deps -- -D warnings is clean."
+++

## Problem

M1's B4 knob `knowledge_section` has no actuator. `HarnessParams::knowledge_section` (`crates/roko-core/src/config/
harness_params.rs:540`, θ₀ = true at :591, notches at :738) is in the knob surface, the move catalog and the SafetyBox
(`crates/roko-learn/src/homeostasis/policy.rs:509`), but `git grep knowledge_section -- crates` finds no dispatch or
prompt code that reads it (only those definitions and unrelated prompt-builder test names). A move on it changes
nothing, yet M1 would log and credit it as if it had.

## Why it matters

Goal `cybernetic` (S06 §4.3, B4). NEW-FINDINGS finding 13; A09 §3 gap 3. A controller whose move does nothing learns
a false effect from noise; in shadow (stage 2) its logged would-be moves on this knob are meaningless. B4's other knob,
`error_patterns_k`, does reach the prompt (`graph_task_dispatch/verification.rs::error_pattern_limit`). Stage 1 of the
epic: wire the actuator before M1 runs in shadow (gap-cad57a).

## Where

- `crates/roko-core/src/config/harness_params.rs::HarnessParams`: the knob.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::withholds`: how the L-know arm already drops the `knowledge`
  section; the θ switch belongs next to it.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs::error_pattern_limit`: how θ reaches per-dispatch prompt
  inputs today (B4's other knob).
- `crates/roko-learn/src/homeostasis/policy.rs::SafetyBox`: the rule that turning the section back on in SEARCH needs a
  live knowledge loop.

## Current state

Decision 8102 accepted the knob surface as specified; the reader was simply never written (8125 wired `error_patterns_k`
and B6/B8 only).

## Plan

1. Carry θ's `knowledge_section` into the prompt context the same way the arm set is carried (per dispatch, from the
   attempt's θ; θ₀ when M1 is not running).
2. When it is false, omit the `knowledge` section exactly as `withholds("knowledge")` does, and log the omission as a
   θ decision (not as L-know's arm) so the census does not credit L-know's holdout.
3. Test `knowledge_section_knob_off_omits_the_knowledge_section`: with θ.knowledge_section = false the assembled prompt
   has no knowledge section and the decision row names θ; with θ₀ it has one.

## Done when

The `[[verify]]` command passes.

## Notes

If wiring proves wrong for some reason, the alternative is to remove the knob from the catalog and the SafetyBox
(S06 change); say so on the epic before doing that.
