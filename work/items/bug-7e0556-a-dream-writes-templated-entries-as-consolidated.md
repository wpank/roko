+++
id = "bug-7e0556"
kind = "bug"
title = "A dream writes templated entries as consolidated knowledge at confidence up to 1.0, and prompts inject them ahead of real lessons, so the next task costs more"
status = "open"
triage = "verified"
severity = "p1"
goal = "learning"
size = "M"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#3.6"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::scored_knowledge", "crates/roko-dreams/src/imagination.rs::synthesize_hypotheses", "crates/roko-dreams/src/hypnagogia.rs::dali_insight", "crates/roko-dreams/src/runner.rs::consolidate_now", "crates/roko-learn/src/loop_audit/loops.toml"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-7257cb", "bug-dee062", "gap-01fb50", "bug-8c5d32"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dream_sourced_knowledge_is_withheld_by_default' crates/roko-cli/ && cargo test -p roko-cli dream_sourced_knowledge_is_withheld_by_default"

[[verify]]
command = "grep -rqw 'fn dream_writes_no_templated_entries_without_model_content' crates/roko-dreams/ && cargo test -p roko-dreams dream_writes_no_templated_entries_without_model_content"
+++

## Problem

One manual dream makes later prompts worse. Repro (round v6, $0, scripted model; evidence
`tmp/hermes-roko-v6/learning-research/evidence/dream/`, `findings/learning-research.md` §3.6):

1. A workspace that ran three similar one-task plans has one real lesson (`runtime_0b57…`, consolidated after three
   confirmations). With it in the prompt, the second plan's task passed on the cheap rung first time for $0.0035.
2. `roko knowledge dream run` makes 3 model calls, all answered "OK.", yet reports "8 knowledge entries". Of those 8,
   7 are written at the **consolidated** tier, 4 of them at confidence **1.0**. Their text comes from templates, not
   from the model:
   - "Combine T1 with T1: reuse the successful routing discipline from one episode in the other."
     (`imagination.rs::synthesize_hypotheses`)
   - "What if T1 had used gpt-oss-120b-counterfactual instead of gpt-oss-120b?"
   - "Dali interrupt: if T1 keeps failing, try a wider search … with the glm-4.7 model."
     (`hypnagogia.rs::dali_insight`)
   - threat and consolidated entries that paste the failed check's output (file paths, test names).
3. Re-running that second plan (`--fresh`): the pasted check output shares 4 topic terms with the task, against the
   real lesson's 2. Ranking is by shared terms first, so all three knowledge slots go to dream entries (score 4.0, see
   `rerun-p2-exposures.jsonl`), and the real lesson drops out of the prompt. The cheap rung fails twice and the task
   climbs: $0.0169 instead of $0.0035 (4.8×).

## Why it matters

Goal `learning`. Round v6 is asked to turn roko's loops on, dreams included. As things stand, a dream fills the store
with high-confidence noise that crowds out verified lessons. The loop registry's own contract for L-dream is
`default_policy = "withhold dream-sourced artifacts"` (`loops.toml`), but prompt assembly does not implement it.
Related:
- bug-7257cb (fabricated journal fields) and bug-dee062 (merges by tags), both held under dec-e70592;
- gap-01fb50 (the dead dream config keys);
- bug-8c5d32 (decay compounds per dream).

## Where

- `crates/roko-cli/src/dispatch/prompt_builder.rs::scored_knowledge` filters group-scoped entries, runtime success
  notes and low confidence, but not `source = "dream"` / `"dream-consolidation"`.
- `crates/roko-learn/src/loop_audit/loops.toml`: the L-dream entry and its default policy.
- `crates/roko-dreams/src/imagination.rs::synthesize_hypotheses`, `crates/roko-dreams/src/hypnagogia.rs::dali_insight`
  and the threat and consolidation writers: entries built from templates and episode fields, then promoted by tier
  progression (`knowledge tier progression … to_tier=Consolidated`).
- `crates/roko-dreams/src/runner.rs::consolidate_now`: the cycle `roko knowledge dream run` drives.

## Current state

- Dreams are off on plan runs (gap-a960a1 removed the plan-completion trigger). They run only by hand or under
  `roko daemon`.
- The harm is real as soon as anyone runs one.
- No arm assignment exists for dream-sourced artifacts. L-dream is registered but nothing withholds or measures it.

## Plan

1. Prompt assembly: withhold knowledge whose `source` is a dream source unless L-dream's arm for the chain is learned
   and the loop is not on probation. Default: withheld, as the registry says. Log the exclusion
   (`excluded_reason = "provenance"`) in `exposures.jsonl`.
2. Dreams: write no entry whose content is a template filled only with ids and model names. A dream entry needs model
   content or verified evidence. Start dream entries at a working tier with the source's discounted confidence, never
   straight at consolidated / 1.0.
3. Tests:
   - `dream_sourced_knowledge_is_withheld_by_default` (roko-cli): a store holding a dream entry with more shared terms
     than a real lesson; the prompt carries the lesson and logs the dream entry as withheld.
   - `dream_writes_no_templated_entries_without_model_content` (roko-dreams): a dream whose model answers are empty or
     "OK." writes no knowledge entries.

## Done when

Both `[[verify]]` commands pass, and the round-v6 repro above leaves the real lesson in the re-run's prompt.

## Notes

The dream items are held under dec-e70592 (Will chose hold over park on 2026-09-29). This item is filed open because
round v6 asks for dreams to be shown; if dreams stay held, put the same hold on this item. Step 1 alone, prompt-side,
already makes dreams safe to run.
