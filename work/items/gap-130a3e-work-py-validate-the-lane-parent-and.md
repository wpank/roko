+++
id = "gap-130a3e"
kind = "gap"
title = "work.py: validate the lane, parent and milestone fields, and add next --lane and --mix"
status = "open"
triage = "unverified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (R1–R3); W7-orchestration-model.md §2, §4"
anchors = ["tools/work.py::validate", "tools/work.py::pick_next", "tools/work.py::cmd_next", "tools/test_work.py", "work/lanes.toml"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f work/lanes.toml && grep -qw 'def test_check_rejects_unknown_lane_and_parent' tools/test_work.py && grep -qw 'def test_next_mix_respects_lane_caps' tools/test_work.py && python3 tools/test_work.py -k test_check_rejects_unknown_lane_and_parent -k test_next_mix_respects_lane_caps"
+++

## Problem

Epics and their items now carry `lane` and `parent` (PLAN.md §2), and the checklist import adds `milestone`, but
`tools/work.py` ignores all three: `validate` accepts any value, and `next` knows nothing of lanes. `next --n 10`
returned ten `release` items in W1's test; it can also return several `rust-hot` items while the `paper` and `bench`
lanes sit idle. Lane caps, such as four Rust builders on the cargo lock, exist only in prose.

## Why it matters

Goal `tooling` (rank 1), epic spec-1e1b45: 6–10 agents at once need a lane-balanced pick. gap-c9e61b (EPICS.md),
gap-2bc1b9 (`new` flags) and gap-25065c (the import) rely on these fields being validated.

## Where

- `tools/work.py`: `validate`, `pick_next`, `cmd_next` and the sort key (`pick_key`, `now_key`).
- **New:** `work/lanes.toml`, hand-edited.
- Tests: `tools/test_work.py`.

## Current state

Checked at `41c7ffbd6`: the E2 sample and today's drafted epics set `lane` and `parent`; nothing validates them.
PLAN.md §2 names the lanes (`paper`, `bench`, `tracker`, `rust-cold`, `rust-hot`, `frontend`, `docs`) and their caps.

## Plan

1. `work/lanes.toml`: one `[lane.<key>]` table per lane with `paths` (globs), `max` (agents at once) and an optional
   `pool`; `[pools] rust = 4`; `milestones = ["MS0", …, "MS6", "ME"]`, in order (the checklist's values).
2. `validate`: `lane` must be a lane key, `parent` an existing `spec` item, `milestone` a listed value.
   `check --lint` warns when an item's anchors fall outside its lane's paths.
3. `next --lane X` picks only lane X. `next --mix "rust-hot=1,rust-cold=2,paper=2"` fills each quota in priority
   order, within each lane's `max` and its pool, counting live claims in that lane.
4. Sort by goal, then milestone, rank and severity.
5. Items without a lane fall in an uncapped `none` lane, so today's items keep working.

## Done when

- [ ] `check` rejects an unknown lane, a parent that is not a spec, and an unknown milestone.
- [ ] `next --mix` never exceeds a lane's `max` or a pool, counting live claims.
- [ ] `check` still reports 0 problems on today's items.
- [ ] The `[[verify]]` command passes.

## Notes

- Will approves the first `lanes.toml`; lane caps are his call (W7, "Will decides").
- Out of scope: W7's per-lane session owners and its pre-commit lane hook.
