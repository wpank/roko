+++
id = "bug-550909"
kind = "bug"
title = "The loop census calls L-play dormant:unlogged and the audit tick calls it reached when every playbook was withheld or dropped by the section bandit"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#4c.15"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-learn/src/loop_audit/census.rs::Logs::verdict", "crates/roko-learn/src/loop_audit/census.rs::Logs::read", "crates/roko-learn/src/loop_audit/exposure.rs"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-ca63ee", "gap-ad9e03"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn log_rule_does_not_call_withheld_playbooks_unlogged' crates/roko-learn/ && cargo test -p roko-learn log_rule_does_not_call_withheld_playbooks_unlogged"
+++

## Problem

In a workspace where no prompt ever included a playbook, the M2 census and the audit tick give two different, both
wrong, accounts of L-play.

Repro (round v6, $0 scripted model, default arms; `tmp/hermes-roko-v6/learning-research/evidence/zero-default/`).
Three one-task plans ran. A playbook was written after runs 1 and 2. In run 2 its prompt's playbook was
`withheld_arm`; in run 3 every playbook, knowledge and episode item was `bandit_excluded` on all 3 attempts
(`workspace/.roko/runs/*/exposures.jsonl`). Run 3's `decisions.jsonl` rows for `knowledge` and `playbooks` say
`chosen: []`, `source: "default"`. Then:
- `roko learn loops` (`03-p3-heading-anchors-after/learn-loops.txt`) prints `L-play  flagged  dormant:unlogged
  (evidence: log)`. Its log rule says "unlogged" because the store holds playbooks while no episode or efficiency row
  carries a playbook id.
- The run-close tick's ledger row for L-play (`workspace/.roko/learn/loop-audit.jsonl`, run 3) says `probation`,
  `n_L 3`, `n_D 1`, `eps.reach 1.0`, `iota.net 0.25`. L-know's row likewise has reach 1.0 although its lesson never
  reached run 3's prompt.

What happened is neither "unlogged" (the effect was logged; there was none to log) nor "reached": no playbook was in any
prompt.

## Why it matters

Goal `cybernetic`. M2's verdicts decide whether a loop counts as live, dormant or harmful. Here the CLI flags a healthy
loop, while the tick counts content the section bandit removed as delivered. The plugin's L4 ring shows the tick's
"reach 100%". Related:
- gap-ca63ee: the census judges with default parameters, a different cause;
- gap-ad9e03: the section bandit dropping `domain_context` is what removed the content.

## Where

- `crates/roko-learn/src/loop_audit/census.rs::Logs::verdict` (log facts gathered in `Logs::read`): the L-play rule is
  `playbooks > 0 && episodes_with_playbooks == 0 && efficiency_with_playbooks == 0`. The L-know rule counts retrieval
  exposures, not inclusions.
- `crates/roko-learn/src/loop_audit/exposure.rs`: ε's read, reach, honest and receipt factors. Reach is "the executed
  action equals the learned one". When the bandit empties the section before the learned proposal is recorded, both
  are empty, so the action counts as reached.

## Current state

Both rules predate the section bandit (L-sec) excluding the section these loops render into.

## Plan

1. Census log rule: call L-play / L-know unlogged only when some prompt *included* an item (exposure rows with
   `included = true`) and no episode or efficiency row carries its id. With items in the store but none included, say
   so as no exposure.
2. Tick: record the loop's learned proposal before section-level exclusion, and count a `bandit_excluded` item as
   executed ≠ learned (not reached), so ε falls when L-sec removes L-know's or L-play's content.
3. Test `log_rule_does_not_call_withheld_playbooks_unlogged`: fixture runs whose playbook exposures are all withheld
   or bandit-excluded give no `dormant:unlogged` for L-play, and a reach below 1.

## Done when

The `[[verify]]` command passes, and the round-v6 repro's census and tick agree that L-play had no exposure in runs 2–3.
