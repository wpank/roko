+++
id = "gap-be6bc5"
kind = "gap"
title = "L-know and L-play's learned proposals leave out the items the token budget cut, so a prompt the budget emptied still reads as reached"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-17
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "df5c3f553"
source = "bug-550909 follow-up 2026-10-17"
discovered_from = "bug-550909"
anchors = ["crates/roko-cli/src/graph_task_dispatch/decision_log.rs::reader_proposals", "crates/roko-learn/src/loop_audit/census.rs::content_opportunity"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["bug-550909", "gap-ca63ee"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn token_budget_cut_items_count_as_not_reached' crates/roko-learn/ && cargo test -p roko-learn token_budget_cut_items_count_as_not_reached"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T04:37:16Z"
commit = "df5c3f553"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-21T01:31:06Z"
forced = false
evidence = "df5c3f553. Decision (plan step 1; see ## Decision): a token-budget cut is the reader's proposal not reached, as the section bandit's exclusions are since bug-550909, and reach keeps S03 §4.4's whole-set comparison, so a partial cut is not reached either. decision_log.rs reader_proposals keeps TokenBudget items in the learned proposal; census.rs kept_out (was bandit_excluded) adds an attempt's token_budget knowledge and playbook exposure items to the learned proposal of rows written before, never to the executed action; CENSUS_STATE_SCHEMA 7 refolds kept censuses. Checks: the verify command exits 0 (token_budget_cut_items_count_as_not_reached: L-know reach 0.25 over chains the budget emptied in rows written before and since the writer change, cut in part, and left whole; L-play 0.5; the tick's ε equals the census's); roko-learn loop_audit 59/59; roko-cli lib decision_log + loop_audit + learn_loops 28/28, incl. reader_rows_propose_what_the_section_bandit_left_out and graph_decisions_log_arm_before_plan_and_receipt, which now expect the cut item proposed; cargo test -p roko-cli --test loop_audit_census_run ok; E1 replay cargo test -p roko-cli --features fault-injection --test loop_audit_faults ok (5 healthy epochs flag no loop; every injected fault found with its reason, canaries at P2/P4/P6 as before); clippy -p roko-learn -p roko-cli --no-deps -D warnings clean; nightly fmt. CLI before and after, on a copy of the round-v6 audit demo's 24 runs in which 7 of L-know's 13 learned-arm prompts lost an item to the budget, written in the old writer's form: roko learn loops gave L-know ε 1.00 (reach 1.0) and β 0.976 [-2.301, 4.254] before, and gives ε 0.46 (reach 0.46) and β not judged (eps_below_min) after; the uncut copy is unchanged (ε 1.00, β 0.976). Report-only: no executed policy changes."
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

## Decision (2026-10-21)

Plan step 1: a budget cut is the reader's proposal not reached. The reader ranked the item in, and the composer's
token budget, which neither L-know nor L-play decides, kept it out, as the section bandit does (bug-550909). Reach keeps
S03 §4.4's whole-set comparison (`1[A_t ≡ A^L_t]`), so a prompt the budget cut only some items from is not reached
either. A share-based reach for sets is not taken: it would change ε for every set-valued loop, not only for budget
cuts.

What to watch: none of the recorded runs has a knowledge or playbook item cut for the token budget yet (the exposure
rows under `tmp/`, round v6's learning-research evidence among them: 52 knowledge and 146 playbook decision points, no
`token_budget` item; the main workspace's `.roko/runs`: none). If partial cuts become common, a healthy loop they hit
on most prompts reads `dormant:mask`, and the share-based reach is the case to reopen.
