+++
id = "gap-6d583a"
kind = "gap"
title = "roko learn self-model fit trains on units without the content group: every offline unit has no L-know or L-play arm and carries no lesson, while live forecasts carry them"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "gap-dd89ff worker, 2026-10-17: the offline fit builds features with TaskFeatures::from(unit), which leaves the content group at its default"
discovered_from = "gap-dd89ff"
anchors = ["crates/roko-learn/src/self_model/ingest.rs::read_runs", "crates/roko-cli/src/commands/learn_self_model.rs::fit_offline"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-dd89ff", "gap-96547f", "gap-b38ed2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn offline_units_carry_the_content_group' crates/roko-learn/ && cargo test -p roko-learn offline_units_carry_the_content_group"
+++

## Problem

`roko learn self-model fit` builds every unit's features with `TaskFeatures::from(unit)`, which leaves the content
group that gap-dd89ff added at its default: no L-know or L-play arm (`knowledge_arm:none`, `playbook_arm:none`) and no
carried knowledge entries or playbooks. Live forecasts carry the chain's arms and what the prompt carried (dispatch
fills the group in `task_features`). So the offline fit trains and scores a model that never sees a lesson, and its
report cannot show whether lessons move the forecasts, the effect gap-dd89ff exists for.

To see it: fit over a workspace whose runs carried stored lessons (`roko learn self-model fit`); every training unit's
`knowledge_carried` is 0, while the same attempts' live prediction rows were forecast from features that carried them
(their `features_hash` differs from the offline unit's features).

## Why it matters

Goal `cybernetic`. The offline fit is how M3's forecasts are checked before anyone promotes it (gap-96547f joins the
shadow forecasts to their verdicts; S04 §7.4). Without the content group it measures a different predictor from the one
that forecasts live, and gap-dd89ff's point, that a lesson-covered task can start on the cheap rung, is invisible to
it. Related: gap-dd89ff, gap-96547f, gap-b38ed2.

## Where

- `crates/roko-learn/src/self_model/ingest.rs::read_runs`: reads `attempts.jsonl` alone (open and verdict lines).
- `crates/roko-learn/src/self_model/model.rs` (`impl From<&Unit> for TaskFeatures`): no content group.
- `crates/roko-cli/src/commands/learn_self_model.rs::fit_offline`: forecasts and observes with `TaskFeatures::from(unit)`.
- The records that hold the group for each attempt, in the same run directory: `decisions.jsonl`'s content decision
  rows for the knowledge and playbooks points carry the chain's draw on L-know's and L-play's layers
  (`graph_task_dispatch/decision_log.rs::content_decisions`), and `exposures.jsonl` holds each retrieved item with
  `included`.

## Current state

Open. gap-dd89ff (2026-10-17) fills the group at dispatch, before routing, from the chain's arm set and the prompt's
readers; the offline path was left unchanged.

## Plan

1. In ingest, join each unit by attempt key to its run's knowledge and playbooks content decision rows (the arms) and
   to its exposure rows (the knowledge and playbook items with `included`, at most `CONTENT_CAP` each).
2. Give the offline `TaskFeatures` that `ContentFeatures`, in `fit_offline` and wherever a unit's own features are
   built (`TaskFeatures::from(&Unit)`), so offline and live units carry the same group.
3. Test `offline_units_carry_the_content_group`: a run directory with one attempt whose knowledge row is on the learned
   arm and whose exposures include one knowledge entry gives a unit whose features carry `knowledge_arm: learned` and
   `knowledge: 1`; one on the default arm, with the entry withheld, gives the default arm and 0.

## Done when

The `[[verify]]` command passes.

## Notes

Ingest is read-only and must stay so. Exposure rows are capped at 64 per attempt (`MAX_EXPOSURES_PER_ATTEMPT`); a unit
whose rows are missing keeps the default group rather than a guess.
