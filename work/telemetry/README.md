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
  events/backfill.jsonl   the items closed before the log existed, reconstructed once (tools/work_backfill.py)
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
| `claim` | `work.py claim` | `by`, `size` (`--size`, else the item's), `claimed_at`; `force` when it took over a claim |
| `release` | `work.py release`, before the claim is deleted | the claim's `by`, `size` and `claimed_at`; `reason` (`--reason`: `verify-fail`, `blocked`, `decision-needed`, `conflict`, `timeout` or `session-limit`) |
| `closed` | `work.py close` (`live`) and `sync` (`reconciled`), in the main checkout | `status`, `commit`, `run_id`, `forced` |
| `merged` | `work.py event merged <id> --merge-sha REV [--conflicts N] [--fixups N]` | `merge_sha`, `conflicts`, `fixups` |
| `post-verify` | `work.py event post-verify <id> --rc N` | `rc`: the exit code of the item's `[[verify]]` on the merged tree |
| `escape` | `work.py event escape <id> [--caused-by ID]` | `caused_by`: the item whose fix let the defect in |
| `intervention` | `work.py event intervention <id>` | none |

A field nobody gave is left out of the row. `release` and `work.py event` take `executor`, `via` and `branch` from the
item's claim when there is one.

## The backfill

`tools/work_backfill.py` (gap-dc6775) reconstructs, once, what the log would have held for the items closed before it
existed, into `events/backfill.jsonl`. Every row has `source` and `session` `backfill`, and the rollup keeps these rows
apart from live ones. It writes one `closed` row per closed item and one `lane-start` row per lane branch whose
creation the reflog still holds; it never edits an item.

A `closed` row's executor comes from the item's `[closed].by` (`plan:` → `roko-plan`; sweep, triage, enrichment and
reconcile closures → `verification-only`; `Will …` → `human`; a session → `claude-session`; a `wk-` worker →
`claude-agent`), else from the `Executor:` trailers of the merge that brought its branch in, else `unknown` with a
`reason`. The row keeps `by`, says which source decided (`executor_from`), and takes `ts` from the commit that
closed the item (`ts_from`). A `lane-start` row has the branch's creation time from its reflog, `created_from`, and
the item when the branch is `work/<id>`.

## Executor fields

`work.py claim --executor claude-agent|claude-session|roko-plan|human --via work-batch|work-next|manual|roko-plan
--size S|M|L` stores the executor, the pick-up route, the size judged before work starts and `claimed_at` in the
claim. `work.py close` copies them into the item's `[closed]` block, with `at_ts` (when it closed, in UTC),
`forced` (whether `--force` overrode a failing verify), and `model` and `assist` (`--model`, `--assist`) when given.
It reads the claim from the main checkout, so a worker closing in its worktree gets them too. `sync` closes an item
from a passed plan task with executor `roko-plan`, and from a commit trailer with the claim's executor, or `unknown`
when nobody claimed it.
