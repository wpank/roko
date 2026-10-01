+++
id = "gap-c56341"
kind = "gap"
title = "plan prepare --full is not built: no LLM-written decomposition.md or rubric.md"
status = "open"
triage = "verified"
severity = "p3"
goal = "features"
size = "M"
subsystem = ["roko-cli/plan"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d6fd85"
anchors = ["crates/roko-cli/src/plan_brief.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d6fd85"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'full' crates/roko-cli/src/plan_brief.rs"
+++

## Problem

gap-d6fd85 added `roko plan prepare` with deterministic companion documents. Its phase 3 is not built: `--full`, an LLM-written `decomposition.md` and `rubric.md`.

## Plan

Build phase 3 as gap-d6fd85's notes describe, behind `--full`.

## Done when

- `roko plan prepare --full` writes both documents, and a test with the scripted provider covers it.

## Notes

- Reported on 2026-10-01 by the worker on gap-d6fd85, during the evening close-out round.
- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `roko plan prepare --full` (`plan_brief::prepare_full`) runs the planner model (`--model`, else
  `[authoring] planner_model`) through `SharedAgentFactory::run_shared_agent_bridge`, read-only (`Read,Grep,Glob`).
  It writes `decomposition.md` from plan.md, tasks.toml and the source PRD, then `rubric.md` from the same plus the
  decomposition. The deterministic `brief.md` and `prd-extract.md` follow, and the brief lists the model-written
  documents. Each document starts with a comment naming its model, and a code fence around the whole reply is dropped.
  Existing documents are kept unless `--force`, and with both kept no model runs. Each call's spend is recorded
  against the plan (`AuthoringSpend`).
- Tests: `prepare_lists_the_model_written_documents` and `document_from_reply_drops_a_fence_around_the_reply` (lib),
  and `tests/plan_prepare_full.rs`, which runs the binary against the shared scripted provider. Its prompts name
  `Task: decomposition:` and `Task: rubric:`, so the provider plays one turn per document.
- Not done: dispatch does not read `decomposition.md` or `rubric.md` into task prompts. The brief, which dispatch
  does read, points to them.
