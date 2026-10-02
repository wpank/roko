+++
id = "gap-e8d97e"
kind = "gap"
title = "docs/v2-depth still presents the removed PRD pipeline as current"
status = "open"
triage = "verified"
severity = "p3"
size = "M"
subsystem = ["docs"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/workflow-audit (roko-7d migration, 2026-10-02)"
discovered_from = "audit:tmp/workflow-audit/"
anchors = ["docs/v2-depth/02-block/enrichment-pipeline.md", "docs/v2-depth/00-index/05-vision-and-positioning.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rlE 'roko prd|/api/prds|auto_plan|PRD publish|PRD published|PrdExtract|PRD [Ee]xtract|PRD Requirements|PRD -> [Pp]lan|\\.roko/prd' docs/v2-depth --include='*.md' | grep -v RESEARCH-PROMPT | grep -q ."
+++

## Problem

`docs/v2-depth/` still describes the removed PRD pipeline as current in 18 files: for example the enrichment
pipeline's PRD step (`02-block/enrichment-pipeline.md`), PRD-driven replanning
(`03-graph/plan-phases-and-actions.md`: "the task list is regenerated from the PRD"), the `/api/prds` routes
(`16-surfaces/05-http-api-and-realtime.md`) and PRD publish subscriptions
(`20-deployment/02-daemon-and-subscription-system.md`). The PRD pipeline, `roko prd`, `/api/prds*` and the
`[prd]` config were removed on 2026-10-02 (`tmp/workflow-audit/`).

## Why it matters

Readers and agents that search the docs find the PRD flow described as the way roko works.

## Where

`grep -rl PRD docs/v2-depth --include='*.md'` (18 files on 2026-10-02, including six `RESEARCH-PROMPT-*.md`
prompts that may be kept as historical inputs).

## Current state

The workflow-audit migration updated `docs/v3/` (72 files), `docs/v2/` (API-REFERENCE, CLI-REFERENCE,
INTEGRATION-GUIDE and the chapter files) and two `docs/v2-depth/` files.

## Plan

For each file: reword the PRD step as "the plan's request or spec" where the mechanism survives (the 8,000-character
source cut now applies to the request), or mark the section historical where it does not (publish, Atelier,
`/api/prds`). Leave `RESEARCH-PROMPT-*.md` as they are, with a one-line historical note.

## Done when

- No non-prompt v2-depth file presents the PRD pipeline as current. Verify:
  `! grep -rlE 'roko prd|/api/prds|auto_plan|PRD publish|PRD published|PrdExtract|PRD [Ee]xtract|PRD Requirements|PRD -> [Pp]lan|\.roko/prd' docs/v2-depth --include='*.md' | grep -v RESEARCH-PROMPT | grep -q .`

## Notes

- Found by the workflow-audit migration (session roko-7d, 2026-10-02). docs/v1 is deprecated and out of scope.
