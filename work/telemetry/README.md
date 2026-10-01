# work/telemetry/: the development record

Not for workers. Nothing here is shown to the agents that do the work, no metric here is a target, and no view
(`NOW.md`, `STATUS.md`, `DRIFT.md`, …), skill prompt or `tools/work.py next` reads it.

This folder records how the backlog gets done: who executed each item, how it was picked up, how long it took, what
it cost, and what happened at merge (epic spec-f2463d). The metrics computed from it are defined in advance in
`DEFINITIONS.md`.

## Layout

```
work/telemetry/
  README.md             this file
  DEFINITIONS.md        the metrics, fixed before any data is analysed
  events/<session>.jsonl  append-only events, one file per session (schema roko.work_event/1)
  harvest/<date>.jsonl  tokens, model and time per item from Claude Code transcripts (tools/work_harvest.py)
```

## Events

`tools/work.py` appends one JSON object per line to `events/<session>.jsonl` in the main checkout. The session is
`--session`, else `$WORK_SESSION`, else the claimant (`--by`), as a slug. Only that session writes its file, so
sessions never conflict, and each file is committed by explicit path like any other file.

A worker in a linked worktree logs nothing: the main checkout's orchestrator or session logs the claim, the release,
the merge and the post-merge verify. Workers never write events or see metrics (W12, honesty rules).

Schema `roko.work_event/1`. Every row has these fields, `null` when unknown:

| Field | Meaning |
|---|---|
| `schema` | `roko.work_event/1` |
| `ts` | when, in UTC to the second (`2026-09-30T08:15:00Z`) |
| `event` | `claim`, `release`, `closed`, `merged`, `post-verify`, `escape`, `intervention` or `lane-start` |
| `item` | the item id (`null` only for a `lane-start` row of a lane branch) |
| `executor` | who executes the item: `claude-agent`, `claude-session`, `roko-plan` or `human` |
| `via` | how it was picked up: `work-batch`, `work-next`, `manual` or `roko-plan` |
| `session` | the session that logged the row; it names the file |
| `branch` | the item's branch, such as `work/<id>` |
| `concurrency` | how many claims were live when the row was logged |
| `source` | `live` (logged as it happened), `harvest`, `reconciled` (derived later, such as a `sync` closure) or `backfill` (reconstructed from history) |

Fields of one event:

| Event | Written by | Fields |
|---|---|---|
| `claim` | `work.py claim` | `by`; `force` when it took over a claim |
| `release` | `work.py release`, before the claim is deleted | `by` (the claimant) |
| `merged` | `work.py event merged <id> --merge-sha REV [--conflicts N] [--fixups N]` | `merge_sha`, `conflicts`, `fixups` |
| `post-verify` | `work.py event post-verify <id> --rc N` | `rc`: the exit code of the item's `[[verify]]` on the merged tree |
| `escape` | `work.py event escape <id> [--caused-by ID]` | `caused_by`: the item whose fix let the defect in |
| `intervention` | `work.py event intervention <id>` | none |

A field nobody gave is left out of the row. `work.py event` takes the branch from the item's claim when there is one.
