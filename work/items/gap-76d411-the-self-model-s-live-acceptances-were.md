+++
id = "gap-76d411"
kind = "gap"
title = "The self-model's live acceptances were dropped when their packages closed: shadow forecasts for 95% of routed attempts (6128), 99% of forecasts joined to verdicts (6129), and the 200-attempt active-mode run (6130)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/self_model", "roko-learn/self_model"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-29"
anchors = ["crates/roko-cli/src/graph_task_dispatch/self_model.rs", "crates/roko-cli/src/runtime_feedback/self_model.rs", "crates/roko-cli/src/dispatch/model_routing.rs::decide_self_model"]
links = { depends_on = [], blocks = [], related = ["gap-d2d750", "gap-7ec3ef", "gap-45c8fe", "bug-78e5ce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "set -- work/history/self-model-live-acceptance-*.md; test -f \"$1\" && grep -Eq 'graph-(run-)?[0-9a-f]+' \"$1\" && grep -q '6128 met' \"$1\" && grep -q '6129 met' \"$1\" && grep -q '6130 met' \"$1\""
+++

## Problem

Three M3 self-model tasks closed on their unit tests, and their live Done-when lines were dropped with no follow-up
item (status-quo assessment 2026-10-15 at 7b6a5af44, finding 29;
`tmp/status-quo/2026-10-15/backlog/A07-61-m3-self-model.md` rows 6128-6130 and lines 82-83):

- **6128:** "One live shadow run of this workspace has a prediction for at least 95% of its routed attempts." No
  `predictions.jsonl` exists anywhere.
- **6129:** "After a live shadow run, `roko learn self-model fit` shows at least 99% of forecast attempts joined to
  their verdicts." `fit` reads no `predictions.jsonl` (`learn_self_model.rs:230-245`), so it cannot show that share
  at all.
- **6130:** "a 200-attempt run logs a source and a propensity on 100% of routed decisions, and its holdout share is
  within ±0.03 of S03's rate." gap-d2d750 closed 6130 on its verify alone.

## Why it matters

Goal `cybernetic`. M3 is one of the papers' four mechanisms; without these runs nothing shows it forecasts, learns
or routes outside unit tests.

## Where

- `crates/roko-cli/src/graph_task_dispatch/self_model.rs` (shadow forecasts), `crates/roko-cli/src/runtime_feedback/self_model.rs`
  (the sink that learns from settled verdicts), `crates/roko-cli/src/dispatch/model_routing.rs::decide_self_model`
  (active routing with propensities), `roko learn self-model fit`.

## Current state

The code exists (A07). Will's 2026-10-15 decision 2 runs the self-model in shadow by default, which makes 6128 and
6129 observable on any plan run. The join measure 6129 needs is a code change that the cybernetic-layer findings
(finding 14: `fit` doesn't read `predictions.jsonl`, calibration is never written) cover; land that first. 6130 needs
a person to set `[self_model] mode = "active"` (decision 6102) and live-run budget, and its calibration gate needs 100
labelled outcomes (finding 14).

## Plan

1. After shadow-by-default and the `fit` join measure land, run one cheap live plan in this workspace and record:
   the share of routed attempts with a prediction (6128, at least 95%) and `fit`'s joined share (6129, at least 99%).
2. When Will approves the budget and active mode, run the 200-attempt check for 6130.
3. Write `work/history/self-model-live-acceptance-<date>.md` with the run ids and one line per task, "6128 met",
   "6129 met", "6130 met", only when it holds. A miss becomes a bug item.

## Done when

The `[[verify]]` command passes.

## Notes

Spend: 6128/6129 ride on any approved cheap run; 6130's 200 attempts need Will's approval (ask with the pilots'
report, per decision 1). The coordinator may split 6130 out as its own held item.
