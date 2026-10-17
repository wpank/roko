+++
id = "gap-be6bc5"
kind = "gap"
title = "L-know and L-play's learned proposals leave out the items the token budget cut, so a prompt the budget emptied still reads as reached"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-550909 follow-up 2026-10-17"
discovered_from = "bug-550909"
anchors = ["crates/roko-cli/src/graph_task_dispatch/decision_log.rs::reader_proposals", "crates/roko-learn/src/loop_audit/census.rs::content_opportunity"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["bug-550909", "gap-ca63ee"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn token_budget_cut_items_count_as_not_reached' crates/roko-learn/ && cargo test -p roko-learn token_budget_cut_items_count_as_not_reached"
+++

## Problem

L-know's and L-play's content rows record as the learned proposal the items the prompt included, plus those the
attempt's arm withheld, θ turned off or (since bug-550909) the section bandit left out. An item the reader retrieved
that the composer then cut for the token budget (`excluded_reason = token_budget`) is in neither the learned proposal
nor the choice. So a learned-arm attempt whose knowledge or playbooks were all cut to fit the budget reads as reached
(∅ = ∅), and one whose items were partly cut reads as reached too (proposal = choice). ε cannot see the budget keeping
a loop's content out of its prompts.

Expected: the learned proposal is what the reader would include, and a budget cut counts as executed ≠ learned, so
reach and ε fall when the budget keeps the loop's items out. Or, if budget cuts are meant to be part of the policy, a
doc comment says so.

## Why it matters

Goal `cybernetic` (M2). ε is the audit's first gate: a loop whose content rarely fits the prompt (a tight role budget,
a long domain context) reads as exposed, and its β is then judged on prompts that never carried its items.
bug-550909 fixed the same blind spot for the section bandit's exclusions; the census's log rule already counts these
items as "cut otherwise" in its no-exposure fact, but the measured ε does not.

## Where

- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::reader_proposals`: the learned proposal keeps items that
  are `included`, or left out as `withheld_arm`, `harness_policy` or `bandit_excluded`; `token_budget` items drop out.
- `crates/roko-learn/src/loop_audit/census.rs::content_opportunity` (with `bandit_excluded`): for rows written before
  bug-550909 the census adds an attempt's `bandit_excluded` exposure items to the learned proposal; `token_budget`
  items are not added.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::ComposedPrompt::excluded_reason`: where an item gets
  `token_budget` (its section was dropped, or its hard cap cut the text).

## Current state

Since bug-550909 (`76ff33984`) the section bandit's exclusions count as proposed and not reached. Token-budget cuts do
not. `section_proposals` (L-sec) takes the budget's cut to fall alike on both arms; for L-know and L-play π⁰ includes
nothing, so the cut does not fall alike there.

## Plan

1. Decide whether a budget cut is the reader's proposal not reached (the reader ranked the item in) or part of the
   learned policy. Reach compares sets exactly, so with the former any partial cut makes the whole opportunity not
   reached; a share-based reach for sets is the alternative.
2. If it is the proposal: add `ExcludedReason::TokenBudget` to `reader_proposals`' filter, have the census treat
   `token_budget` exposure items like `bandit_excluded` ones for older rows, and bump `CENSUS_STATE_SCHEMA`.
3. Test `token_budget_cut_items_count_as_not_reached`: a learned-arm L-know row whose items were all cut for the token
   budget gives a reach below 1 in the census and the tick.

## Done when

The `[[verify]]` command passes.

## Notes

Report-only: no executed policy changes. Re-run the E1 replay (`cargo test -p roko-cli --features fault-injection
--test loop_audit_faults`): healthy L-know must stay above ε_min wherever the budget cuts some of its items.
