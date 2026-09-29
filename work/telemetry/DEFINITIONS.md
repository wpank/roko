# Development record: metric definitions

Version 1 · 2026-09-29

Not for workers. Nothing under `work/telemetry/` is shown to the agents that do the work, and no metric here is a
target.

These are the item-level metrics of the development record (epic spec-f2463d): how each backlog item got done, by
whom, at what cost, and what broke at merge. They are fixed before any data is analysed, so no figure can be tuned to
its result. The daily rollup (`tools/work_telemetry.py`, gap-ccb87e) computes each metric exactly as written here and
prints this file's sha256 at the top of every report. A change adds a version and a change-log entry; old entries are
never rewritten, and every report names the version it used.

The metrics describe backlog items. S01's autonomy index and the other run-level figures of `field_rollup.py`
describe Roko plan runs. They are a different record, and no metric here reuses their names.

## Conventions

- **Time.** All times are UTC. A merged item belongs to the day, and to the reporting window, of its first merge.
- **Figures.** A rate is shown with its numerator and denominator. Durations and costs are reported as median and
  p90, never as a mean alone.
- **Executor.** The claim's `executor`: `claude-agent` (a worker spawned by a work skill), `claude-session` (a session
  that did the item itself), `roko-plan` (a Roko plan run) or `human`. `assist` names a second executor that helped.
- **Labels.** Every figure is "observational". Costs of Claude sessions are "API-equivalent (subscription)". Rows
  written from history after the fact carry `source = "backfill"`: they are tabled apart and never pooled with live
  rows. Pilot data (S09) is kept apart too.
- **Missing inputs.** An item whose trail lacks an input that a metric reads is left out of that metric's denominator
  and counts against its coverage. A definition never changes to fit the data that happens to exist.

## Inputs

| Input | Written by | Fields read |
|---|---|---|
| Events, `work/telemetry/events/*.jsonl`, schema `roko.work_event/1` | `work.py` (gap-d0643c, gap-0b9056); the work skills (gap-92033c); the backfill (gap-dc6775) | `ts`, `event`, `item`, `executor`, `via`, `session`, `branch`, `source`; `claim`: `size`, `claimed_at`; `release`: `reason`; `merged`: `merge_sha`, `conflicts`, `fixups`; `post-verify`: `rc`; `escape`: `caused_by` |
| The item's `[closed]` block and `status` | `work.py close` (gap-0b9056) | `at`, `at_ts`, `executor`, `via`, `size`, `claimed_at`, `assist`, and whether the close was forced |
| Commit trailers | the work skills (gap-92033c); whoever fixes a regression | `Work-Item:`, `Executor:`, `Conflicts:`, `Post-Merge-Verify:`, `Fixes:` |
| Harvest rows, `work/telemetry/harvest/<date>.jsonl`, schema `roko.work_harvest/1` | `tools/work_harvest.py` (gap-263de5) | `calls` rows: `item`, `join`, `branch`, `origin`, `model`, `first_ts` and the token counts |
| Field notes with `--item` | `field_note.py` (bug-7b37c4) | `item`, `kind`, `ts` |
| Prices | snapshot `prices-2026-09-28`, `config/prices/2026-09-28.toml` | per model: `input`, `output`, `cache_read`, `cache_write_5m`, `cache_write_1h` |
| Roko's ledger (Phase B) | Graph plan runs, `.roko/state/graph/<plan>/costs.json` | the recorded cost of each task |

## Metrics

### Merged item

The unit that the other metrics count. An item is **merged** when its work reached the working branch through a
merge of its branch: its first `merged` event or, where no event was logged, a merge commit with a `Work-Item: <id>`
trailer. A Roko-executed item is merged when its plan branch merges (W12 B2). The item must be `done` when the rollup
runs.

- **Reads:** `merged` events (`item`, `ts`, `merge_sha`); `Work-Item:` trailers; the item's `status`.
- **Not merged:** items closed `wontfix`, `superseded` or `parked`; verification-only closures, where a sweep or
  triage found the work already done; and items closed `done` without a merge, such as a direct commit on the working
  branch. The last group is listed under Coverage.

### Attempts

Per merged item, the number of `claim` events for it up to its first merge. Every claim counts, including one that
takes over a dead claim (`claim --force`) and one that follows a release. A holder renewing its own live claim
(`claim --renew`) does not count.

- **Reads:** `claim` events (`item`, `ts`, `executor`, `session`); `merged` events.
- **Excludes:** items with no `claim` event, merged outside the skills or backfilled.
- **Reported as:** the distribution (1, 2, 3 or more) and the mean, by executor.

### First-try merge

A merged item is a **first-try merge** when all three hold:

1. It took one attempt: a single claim, never released before the merge.
2. The worker's verify passed. The worker closed the item on its branch without `close --force`: `close` refuses a
   `done` close whose static verify fails unless forced, and the worker runs the full verify before closing. A worker
   whose verify fails is released with reason `verify-fail`, which breaks condition 1. A Roko-executed item passes
   this condition when `sync` closed it from a passed plan task: `sync` closes only for a passed task, and only when
   the item's verify passes.
3. The post-merge verify passed: its `post-verify` event has `rc = 0`, or its merge commit carries
   `Post-Merge-Verify: pass`.

Share = first-try merges ÷ merged items with all three inputs recorded.

- **Reads:** `claim` and `release` events; `[closed]` (whether forced); `post-verify` events (`rc`);
  `Post-Merge-Verify:` trailers.
- **Excludes:** items missing any of the three inputs.

### Claim-to-merge hours

Per merged item, the hours from the `ts` of its first `claim` event to the `ts` of its first `merged` event. The span
covers every attempt, including the wait between a release and the next claim.

- **Reads:** `claim` and `merged` events (`ts`).
- **Excludes:** items with no claim or no merged event. Backfilled `lane-start` rows are shown apart and never mixed
  in.
- **Reported as:** median and p90, by executor and by size.

### Conflict rate

A merge **conflicted** when git reported at least one conflicted file: its `merged` event has `conflicts ≥ 1`, or
its merge commit's `Conflicts:` trailer is not 0.

Rate = conflicted merges ÷ merges with a recorded conflict count, in the window. Merges abandoned over a conflict
(`release` with reason `conflict`) are counted and shown beside the rate.

- **Reads:** `merged` events (`conflicts`); `Conflicts:` trailers; `release` events (`reason`).
- **Note:** a conflicted merge is not by itself an intervention; fixup commits are (see Intervention).

### Post-merge verify failure

Before committing a merge, the skill runs the item's `[[verify]]` on the merged tree. That run **fails** when it exits
non-zero: a `post-verify` event with `rc ≠ 0`, or `Post-Merge-Verify: fail`.

Rate = failed post-merge verifies ÷ post-merge verifies run, in the window. Every run is logged as a `post-verify`
event, pass or fail. A failure logged only as a release cannot be told apart from a failed worker verify, so it is
not counted here.

- **Reads:** `post-verify` events (`item`, `ts`, `rc`); `Post-Merge-Verify:` trailers.

### Escape

A merged item **escaped** when, within 14 days of its first merge, any of these happens:

1. Its `[[verify]]` fails when re-run on the working branch (the weekly `work.py verify --closed-since`), and fails
   again on an immediate second run. This is logged as an `escape` event.
2. A `reg-*` item names it as the cause (an `escape` event with `caused_by`).
3. A commit whose `Fixes:` trailer names it lands.
4. A field note with `--item <id>` and kind `escape` records a defect found in it after its gates passed.

Rate = escaped items ÷ merged items whose 14-day window has ended by the report date. Items merged within the last 14
days are shown as pending and are not counted.

- **Reads:** `escape` events (`item`, `ts`, `caused_by`); `Fixes:` trailers; field notes (`item`, `kind`, `ts`);
  `merged` events (`ts`).
- **Excludes:** a verify failure that does not repeat on the immediate second run (a flaky check).

### Intervention

An **intervention** on an item is any of these, from its first claim to its first merge, including the merge itself:

1. a commit on the item's branch whose `Executor:` trailer differs from the claim's `executor`;
2. a fixup commit in its merge (the `merged` event has `fixups ≥ 1`);
3. a forced close (`close --force`);
4. a `release` with reason `decision-needed`;
5. an `intervention` event for it (`work.py event intervention`);
6. a field note with `--item <id>` and kind `intervention`;
7. a second executor named in `[closed].assist`.

Each signal counts once. Interventions per merged item is the count; the share of items with at least one is shown
beside it.

- **Reads:** `Executor:` trailers and the claim's `executor`; `merged` events (`fixups`); `[closed]` (whether forced,
  `assist`); `release` events (`reason`); `intervention` events; field notes (`item`, `kind`).
- **Excludes:** a conflicted merge by itself, and the orchestrator's merge and bookkeeping commits, which are the
  process rather than help with the item.
- **Flagged:** a branch commit with no `Executor:` trailer is not counted as an intervention. It makes the trail
  incomplete, and the rollup lists it. The rollup also lists every forced close, release or other-executor commit
  that has no field note (W12, honesty rules).

### Unassisted merge share

Share = merged items with no intervention ÷ merged items with a complete intervention trail, in the window.

This is an item-level measure: the share of the backlog that merged without anyone stepping in. It is not S01's
autonomy index, a run-level ratio of automatic recoveries to interventions, and it is never reported under that name.

- **Reads:** everything that Intervention reads.
- **Excludes:** items with an incomplete trail: no claim event, no record of whether the close was forced, or a branch
  commit without an `Executor:` trailer.

### Cost per merged item

Per merged item, the API-equivalent cost of every Claude call joined to it, over all its attempts, up to its first
merge.

- **Calls:** harvest `calls` rows with `item = <id>` and a `first_ts` at or before the first merge's `ts`. These come
  from every session and agent, released attempts included, whatever the join (`branch`, `prompt` or `item-line`).
  Tables show each item's tokens by join.
- **Price:** tokens × the `prices-2026-09-28` rate of the row's model, per class: `input_tokens` × `input`,
  `output_tokens` × `output`, `cache_read_tokens` × `cache_read`, `cache_write_5m_tokens` × `cache_write_5m`,
  `cache_write_1h_tokens` × `cache_write_1h`. A model id that carries a date or context suffix is priced as its base
  id (`claude-haiku-4-5-20251001` as `claude-haiku-4-5`). A model that the snapshot does not list is not priced: it
  is `null`, with its cost source unknown (S08), and gets no proxy rate. Tables state the share of tokens left
  unpriced. Web search and fetch requests are counted, not priced.
- **Label:** "API-equivalent (subscription)". The sessions run on a subscription, so this is what the same tokens
  would cost on the API, not money spent.
- **Roko-executed items (Phase B):** Roko's ledger as recorded, plus the calls of the operator sessions that
  supervised the plan run (harvest rows on the plan's branch with origin `operator`). Harvest rows with origin
  `roko`, Roko's own `sdk-cli` children, are already in the ledger. They are used only to reconcile it (T9) and are
  never added.
- **Orchestration overhead** has its own row and is never added to an item's cost. It is the harvest `calls` rows with
  no item (`join = none`) in the window, by origin: a total, and that total divided by the items merged in the window.
- **Reported as:** median and p90 by executor, and the total per window.
- **Excludes:** calls after the first merge (rework and escape fixes), which are reported with Escape; items with no
  harvest row joined to them.

### Coverage

Per metric and per table, **coverage** = merged items whose trail holds every input that metric reads ÷ all merged
items in the window.

A full trail is:
- a `claim` event with an executor;
- the `merged` event or a `Work-Item:` trailer;
- a `post-verify` event or a `Post-Merge-Verify:` trailer;
- a `[closed]` block with an executor and a record of whether the close was forced (a `sync` close never is);
- for a Claude-executed item, at least one harvest row joined to it.

Every table states its coverage as "k of n merged items (p%)". It also lists apart the items closed `done` without a
merge and the items merged outside the skills (no claim).

- **Reads:** every input above.

### The switch

The **switch** is the move from Claude-executed to Roko-executed items (W12 B1). Its point is fixed in advance: the
closing commit of the tracker item "Roko executes work items", which W12 B1 proposes and which is not yet filed.
The "before" arm is the Claude-executed items merged before that commit. Items merged after it are compared by
executor: Roko (`roko-plan`) against Claude (`claude-agent`, `claude-session`). Roko-executed items merged before the
switch are reported apart.

- **Compared:** first-try merge share, cost per merged item, interventions per merged item, and attempts.
- **Shape:** an interrupted time series by week, from the first live event to the latest report.
- **Strata:** kind × size × goal, each shown with its counts. A stratum with fewer than 3 items in either arm is
  pooled into "other".
- **Label:** descriptive. The item mix, harness fixes and the operator's own learning all change around the switch,
  so the comparison is not causal. The one exception is a seeded draw (W12 B4): eligible items (size S, cold files, a
  sound verify) are assigned to executors by a draw recorded before the work starts. Within that set the comparison
  may be called causal, and it is reported apart.
- **Reads:** the executor on every event and `[closed]` block; the switch commit; everything the compared metrics
  read.

## Change log

- **Version 1 · 2026-09-29.** First version, written before any data is analysed (gap-7984a5, epic spec-f2463d, from
  W12 A6).
