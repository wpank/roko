+++
id = "gap-c9e61b"
kind = "gap"
title = "work.py list, show and status, plus a generated EPICS.md with progress per epic and lane"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "a8ecec27d"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (§1, R3, R4)"
anchors = ["tools/work.py::render_root", "tools/work.py::main", "tools/test_work.py", "work/EPICS.md"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = ["gap-130a3e"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_epics_view_counts_children' tools/test_work.py && grep -qw 'def test_show_lists_children_and_dependents' tools/test_work.py && python3 tools/test_work.py -k test_epics_view_counts_children -k test_show_lists_children_and_dependents"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:53:24Z"
commit = "a8ecec27d"
by = "wk-gates"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-01T08:36:21Z"
model = "claude-opus-5-5"
forced = false
evidence = "Premise re-checked at bdeaff586: no list, show or status command and no EPICS.md; epics are 'Epic: …' spec items with depends_on on their children, and 343 items set parent. a8ecec27d: read-only list (filters goal/lane/kind/status/parent), show (front matter, anchors, children = parent-of plus an epic's depends_on, dependents, live claim, verify commands, [closed]) and status (open/claimed/done by goal, lane, epic), each with --json; render writes work/EPICS.md per epic (closed/total children, open by lane, the child next takes first with claims ignored and dependencies judged on all items) plus a per-lane table; README layout and views list updated. On real data: 18 epics, e.g. spec-ae5f94 20/24 closed. tools/test_work.py: a three-child epic with one closed child shows 1/3 and lane counts x 1, y 1 and its next child; show lists children, dependents and the claim; list --lane and --parent filter; status counts by epic and lane; the [[verify]] command passes."
+++

## Problem

`tools/work.py` has 17 subcommands but no way to look at one item or a slice of the graph: there is no `list`, `show`
or `status` (W1 §1), so agents grep item files and parse TOML by hand. Epics track their children through
`depends_on` and a checklist in the body, and nothing shows how far an epic or a lane has got.

## Why it matters

Goal `tooling`, epic spec-1e1b45. PLAN.md §2 promises a generated `EPICS.md` (done out of total, per epic and per
lane) as the progress view for the whitepaper and Roko work. Workers and whoever merges need `show <id>` to see an
item with its children, dependents and claim.

## Where

- `tools/work.py`: `main` (parsers), `render_root` (writes the views), `load` and `index`.
- **New:** `work/EPICS.md`, generated; like the other views it is regenerated only in the main checkout.
- Tests: `tools/test_work.py`.

## Current state

Checked at `41c7ffbd6`: none of this exists. Epics are `kind = "spec"` items titled "Epic: …" whose `depends_on`
lists every child; children set `parent`, which gap-130a3e validates.

## Plan

1. `list [--goal G] [--lane L] [--kind K] [--status S] [--parent ID] [--json]`: one line per item.
2. `show <id> [--json]`: front matter, children (items whose `parent` is the id, plus its `depends_on`), dependents,
   the live claim and the verify commands.
3. `status [--json]`: open, claimed and done counts by goal, lane and epic.
4. `render` also writes `work/EPICS.md`: for each epic, done out of total children (the union of `parent` and
   `depends_on`), open children by lane, and the child `next` would pick first; then done out of total per lane.
5. All three commands are read-only; none prunes claims.

## Done when

- [ ] For a fixture epic with three children, one done, `EPICS.md` shows 1/3 and the lane counts.
- [ ] `show` lists children, dependents and the claim; `list --lane` filters by lane.
- [ ] The `[[verify]]` command passes.

## Notes

- Add `EPICS.md` to the README's layout and to the list of views that branches never commit.
- Waits for gap-130a3e.
