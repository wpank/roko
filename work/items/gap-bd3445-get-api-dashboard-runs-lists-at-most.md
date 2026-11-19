+++
id = "gap-bd3445"
kind = "gap"
title = "GET /api/dashboard/runs lists at most 128 runs in directory order, sorted by id, with no start or end times and no paging, so a host cannot list a project's newest runs"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/runs"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round v7 plugin-wow, P-56 (hermes-plugin-roko 60d5ade), the lead's ask"
discovered_from = "agent:plugin-wow (round v7, the pilot's P-51/P-53 chat windows)"
anchors = ["crates/roko-serve/src/routes/runs.rs::get_dashboard_runs", "crates/roko-serve/src/routes/runs.rs::discover_indexed_runs"]
links = { depends_on = [], blocks = [], related = ["bug-c872f6", "bug-df0e79"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dashboard_runs_lists_the_newest_first_with_their_times' crates/roko-serve/ && cargo test -p roko-serve dashboard_runs_lists_the_newest_first_with_their_times"
+++

## Problem

`GET /api/dashboard/runs` is the one route that lists a workspace's runs, and a host can't use it to show a
project's recent runs:

- **It returns at most 128 runs.** `MAX_DASHBOARD_RUNS` caps it, after at most 512 index files
  (`MAX_DASHBOARD_INDEX_ENTRIES`) and 8 MiB read. The index files are visited in `read_dir` order of
  `.roko/events-by-run/`. Their names are a sha256 of the run id, so that order is effectively random.
  `truncated: true` says runs were left out, but not which ones.
- **It sorts by run id, not by time** (a `BTreeMap`).
- **Its rows have no start or end time.** A row is `run_id, template, prompt, current_phase, phases_visited,
  gates_passed, gates_failed, agents_spawned, is_complete, outcome, source, truncated`. `summarize_events`
  computes `started_at` and `finished_at` for each row it builds; the row drops them.
- **It has no paging:** no `limit`, and no `before` or cursor.

So in a workspace with more than 128 runs, the newest can be missing. In any workspace, a client has to ask
each run's summary or bundle just to learn when it ran.

## Why it matters

A host needs "this project's recent runs, newest first", including runs it didn't start: those from roko's CLI or
from another client.

- Round v7's P-53 window shows why. A Hermes chat answered about another project's failed run, because its
  overview listed none of the test project's runs. Those runs had been started over MCP by a harness.
- hermes-plugin-roko's fix (P-56, plugin commit 60d5ade) had to work around this route:
  - it reads the workspace's run manifests itself (`.roko/runs/<id>/manifest.json`, the 8 written last);
  - it asks `run_status` for each one, at 3 s per run and 6 s in all.
  - That works only beside the serve, on its filesystem, and never for a remote serve.
- The plugin's `find_new_run` (recovering a paid start that timed out) reads this route and then each
  candidate's bundle, for the same reason.
- The portal, the TUI and an ACP client would need the same list.

## Where

- `crates/roko-serve/src/routes/runs.rs`:
  - `get_dashboard_runs` is the route;
  - `discover_indexed_runs` does the walk: the caps, the `read_dir` order, the row fields, and the `BTreeMap` by
    run id.
- `roko_fs::run_index::run_index_path` names each run's index `.roko/events-by-run/<sha256(run_id)>.jsonl`.
- The run manifests hold what a list needs without reading any index: `.roko/runs/<run>/manifest.json`, with
  `invocations[].started_at`, `closed.ended_at` and `closed.status`, `origin` and `plan_ids`.

## Current state

At fe204e330 (the route last changed in d1cd5994d), the route behaves as described under Problem. Listing also
settles a run whose process is gone (`interrupted_runs::settle`), which should stay.

## Plan

1. **Rows carry times and origin:** `started_at`, `finished_at` (`summarize_events` already has them),
   `plan_ids`, and `origin` (`cli`, `http`, `mcp:<client>`), from the manifest.
2. **Newest first, by start time.** To find the newest without reading every index, walk the runs by their
   manifests' (or index files') modification time, newest first, before the caps apply.
3. **Paging:** `?limit=N` (default 50, at most 128) and `?before=<started_at>` (or an opaque cursor), with the
   next page's `before` in the answer. `truncated` keeps meaning "this page was cut".
4. Keep the scan caps, per page.

## Done when

- A workspace with 200 runs answers its 50 newest, newest first, each with `started_at` and `finished_at`;
  `?before=` gives the next 50.
- The verify test checks that.

## Notes

- A terminal run's records and events have different ids (bug-c872f6), so a list built from manifests names
  the records id. Agree with that item which id a row gives.
- Once this lands, hermes-plugin-roko can read the route instead of the manifests (P-56's
  `views.outside_runs`, `rundata.other_runs`).
