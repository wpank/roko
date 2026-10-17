+++
id = "reg-48b7ba"
kind = "regression"
title = "The route-inventory snapshot lists every registered route again, including GET /runs/{run_id}/diff, so CI's route check passes"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["tools", "roko-serve/routes"]
created = 2026-10-15
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "486f2c521"
source = "status-quo-2026-10-15#finding-1"
anchors = ["tools/http_route_inventory.snapshot.json", "tools/http_route_inventory.py::check_snapshot", "crates/roko-serve/src/routes/runs.rs::get_run_diff"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["bug-d1dea8", "find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/http_route_inventory.py --check-snapshot"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T05:21:35Z"
commit = "486f2c521"
by = "coordinator 2026-10-17"
executor = "claude-session"
via = "manual"
forced = false
evidence = "At 486f2c521, tools/http_route_inventory.py --refresh adds exactly GET /runs/{run_id}/diff -> get_run_diff (crates/roko-serve/src/routes/runs.rs::routes), public since 4bc447702 (docs/v3/26-HTTP-API.md, openapi.rs): registrations 485 -> 486, canonical 425 -> 426, all other snapshot changes are line-number shifts. --check-snapshot: OK (486 registrations, snapshot matches); tools/test_http_route_inventory.py: 24 tests OK. Plan step 4 (run --check-snapshot in the local merge step) is a change to the local .claude/skills/work-batch skill, left for Will."
+++

## Problem

`python3 tools/http_route_inventory.py --check-snapshot` fails at HEAD (b42578bab; also at 7b6a5af44, where the
2026-10-15 assessment ran it):

```
FAIL: route inventory check failed:
  - route added: GET /runs/{run_id}/diff -> get_run_diff (crates/roko-serve/src/routes/runs.rs::routes); run --refresh to update
  - canonical registration count changed: snapshot=425 current=426; run --refresh to update
```

`4bc447702` (2026-10-13, "runs: an ended run's diff, for chat hosts to show what it changed") added the route at
`crates/roko-serve/src/routes/runs.rs:59` without refreshing `tools/http_route_inventory.snapshot.json`.
bug-d1dea8 (done 2026-10-01 in `bb04bcc91`) made this check a CI gate, and its `[[verify]]` is this same command, so
a done item's verify fails again: a regression.

## Why it matters

CI's `route-inventory` job (`.github/workflows/ci.yml:207-218`) goes red on the next push of local main, which is
97 commits ahead of `origin/main`. Decision 11 (2026-10-15, `tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`):
after this fix, push a branch and open a PR so CI runs on HEAD, then require checks on main (find-8cc7ac). This
item is the first step of that.

## Where

- `tools/http_route_inventory.snapshot.json`: the committed route list (425 canonical registrations).
- `tools/http_route_inventory.py::check_snapshot` (`--check-snapshot`) diffs the scanned routes against it, ignoring
  line numbers; `write_snapshot` (`--refresh`) rewrites it.
- `crates/roko-serve/src/routes/runs.rs::get_run_diff`: the new handler (`GET /api/runs/{run_id}/diff`).

## Current state

The gate works: it names the added route. Only the snapshot is stale. Nothing local runs the check before a merge:
the `/work-batch` merge step (`.claude/skills/work-batch/SKILL.md`, step 5) runs each item's own verify only, and a
direct commit such as `4bc447702` runs nothing. CI would have caught it, but main has not been pushed since.

## Plan

1. Check that the route is meant to be public: `4bc447702` documented it in `docs/v3/26-HTTP-API.md` and
   `crates/roko-serve/src/openapi.rs`, so it is.
2. `python3 tools/http_route_inventory.py --refresh`, read the snapshot diff (it should add `GET /runs/{run_id}/diff`
   and nothing else; name any other route it adds in the commit message), and commit the snapshot.
3. `python3 tools/test_http_route_inventory.py` still passes.
4. Optional, to stop the next one: run `--check-snapshot` in the local merge step when a branch touches
   `crates/roko-serve/src/` (work-batch step 5, next to the post-verify). Trade-off: one more cheap step per merge,
   against the snapshot drifting unseen for as long as main goes unpushed. If you do it, it is a tracker-lane change
   to `.claude/skills/work-batch/SKILL.md`; otherwise file it.

## Done when

`python3 tools/http_route_inventory.py --check-snapshot` exits 0 at the fix commit (the `[[verify]]` command), and
the committed snapshot diff adds only the routes registered since `bb04bcc91`'s refresh.

## Notes

- Reopens bug-d1dea8 as a regression (decision 7, 2026-10-15). It must land before the decision-11 PR.
- Don't hand-edit the snapshot; `--refresh` writes it.
- Evidence: `tmp/status-quo/2026-10-15/backlog/B-top-level-and-regressions.md:164` and `:194`; re-run at b42578bab
  by agent F1 (still fails, same two lines).
