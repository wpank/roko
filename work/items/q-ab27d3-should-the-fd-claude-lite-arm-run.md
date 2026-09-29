+++
id = "q-ab27d3"
kind = "question"
title = "Should the fd_claude_lite arm run Claude Sonnet 5.5, which is priced the same as Sonnet 5?"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench", "config/prices"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:46, wk-bench-tree's report on gap-0580f7)"
anchors = ["config/prices/2026-09-28.toml", "tmp/cybernetic-harness/specs/S09-experiments.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-0580f7", "dec-39c781"], supersedes = [], duplicate_of = "" }
+++

## Problem

Decision D2 (S09 v1.1, `specs/S09-experiments.md:3`, `:154`, `:635`) pins the frontier-lite arm `fd_claude_lite` to
`claude-sonnet-5`, and the price snapshot has a row for it (`config/prices/2026-09-28.toml:24`: $2 input, $2.50 and
$4 cache writes, $0.20 cache hits, $10 output). Anthropic's pricing page (fetched 2026-09-29) also lists **Claude
Sonnet 5.5** at exactly the same rates, with a smaller tool-use system prompt (286 tokens against 354). The snapshot
has no Sonnet 5.5 row.

## Why it matters

`fd_claude_lite` is the "should" arm that shows whether a cheaper frontier model already closes the gap (S09: 30 tasks
× 3 seeds, budget line BL1). If a newer model at the same price is what users would pick, the arm tests an
outdated baseline. The choice has to be settled before the pre-registration lock, which freezes the model ids.

## Where

- `config/prices/2026-09-28.toml`: immutable. Adding a model means a new dated snapshot and a new `price_snapshot_id`.
- S09 (D2 lines and the arms table), S08's arm list, S10, the research paper's §5, §6 and App. A, and
  `paper/FIGURES-TABLES.md` all name `claude-sonnet-5` for this arm.

## Current state

No pilot arm uses `fd_claude_lite`: Pilot A runs `cheap_direct` and `fd_api`, and Pilot B the Roko and Claude Code
arms. The question can wait until the pre-registration lock.

## Plan

The options:

1. **Keep `claude-sonnet-5`** (as decided). Nothing changes.
2. **Move the arm to Sonnet 5.5.** Add a dated price snapshot with a Sonnet 5.5 row (the old file stays), then update
   D2 in S09, the arms tables and the paper's mentions.

The default is option 1 for the pilots, with the question settled at the pre-registration lock (dec-39c781).

## Done when

- [ ] Will has chosen, and D2 in S09 and `DECISIONS.md` records the choice.
- [ ] If the arm moves, a new dated price snapshot exists, and every mention of the arm's model is updated.

## Notes

- Never edit `config/prices/2026-09-28.toml`: its header says it is immutable.
