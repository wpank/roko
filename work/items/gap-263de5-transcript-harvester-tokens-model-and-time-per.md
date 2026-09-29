+++
id = "gap-263de5"
kind = "gap"
title = "Transcript harvester: tokens, model and time per backlog item from Claude Code transcripts"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (A4, F2)"
anchors = ["tools/work_harvest.py", "tools/test_work_harvest.py", "work/telemetry/harvest/"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_harvest_joins_calls_to_items_by_branch' tools/test_work_harvest.py && grep -qw 'def test_harvest_keeps_no_message_content' tools/test_work_harvest.py && python3 tools/test_work_harvest.py -k test_harvest_joins_calls_to_items_by_branch -k test_harvest_keeps_no_message_content"
+++

## Problem

Nothing records what the Claude side of the work costs. Claude Code transcripts already attribute every API call
(`sessionId`, `gitBranch`, `entrypoint`, `timestamp`, and for assistant turns `message.model` and `message.usage`),
but nothing reads them, so tokens and time per work item are unknown.

## Why it matters

Goal `proof`, epic spec-f2463d. W12 estimates the Claude sessions of 09-25..29 at about $2.7–3.4k API-equivalent,
against $172.80 that Roko recorded. Total cost including the operator, and cost per merged item, need this per item.

## Where

- **New:** `tools/work_harvest.py` (standard library only) and `tools/test_work_harvest.py` (`unittest`, synthetic
  transcripts in a temp dir).
- **New output:** `work/telemetry/harvest/<date>.jsonl`.
- Input: every `~/.claude/projects/-Users-will-dev-nunchi-roko-roko*` directory (20 on 09-29, including worktree
  sessions): session files plus `<session>/subagents/*.jsonl`.

## Current state

Checked at `41c7ffbd6`: no harvester. W12 found that every deduplicated subagent call since 09-25 carries
`gitBranch`, and that `entrypoint` separates operator sessions (`cli`, `claude-desktop`) from Roko's own ClaudeCli
children (`sdk-cli`).

## Plan

1. Walk the directories, parse lines defensively, and count each call once by `message.id` (streaming writes a line
   per content block).
2. Keep derived fields only (D27): time, session id, `entrypoint`, `isSidechain`, branch, model and token counts.
   Never message text, tool input or tool output.
3. Join to items on `gitBranch = work/<id>`; else an item id found in the session's first prompt or a worker's `ITEM:`
   line. Store only the id.
4. One row per (date, session, branch, item, model, entrypoint): call and token sums, first and last timestamp. Calls
   with no item go to an `overhead` row for their branch.
5. Incremental and idempotent: keep file offsets outside `work/` (for example `.roko/work-harvest/state.json`).

## Done when

- [ ] A fixture of two sessions and a subagent on `work/gap-aaaaaa` yields one item row with summed tokens; repeated
      lines of one message count once.
- [ ] No output row contains message or tool text.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs Will (W12 risk 2):** may harvest rows (session ids, branch names, token counts; no content) be committed to
  the public `work/` under D27? Until he says yes, write them but do not commit them.
- Sum per-call `message.usage` from the subagent files; whether an Agent result's `usage` covers the whole agent is
  unchecked (W12 risk 4).
- **Decided 2026-09-29 (Will):** commit the harvested rows (counts and session ids, no content) under `work/telemetry/`.
