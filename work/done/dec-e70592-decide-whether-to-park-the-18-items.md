+++
id = "dec-e70592"
kind = "decision"
title = "Decide whether to park the 18 items the TL;DR says to drop"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["work/items"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "70820a74c"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§5, Park); tldr/05-GAPS-AND-PROPOSALS.md (§3; §6 decisions 12 and 13)"
anchors = ["work/parked/"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-470de8"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "Will (decided 2026-09-29)"
evidence = "Will decided 2026-09-29: option 3, hold all 18 rather than park them. Each now carries a hold line citing this decision; removing it revives the item."
+++

## Problem

tldr/05 §3 takes these subsystems out of the default build and the pitch: dreams, affect, the conductor, the neuro
modules beyond the store, LinUCB tuning, and Roko's own channel layer. `PLAN.md` §5 lists the 18 open items that belong
to them. While they stay open, `work.py next`, `NOW.md` and `STATUS.md` keep offering them to agents. Should they be
parked?

## Why it matters

Agents would otherwise spend time on subsystems the plan has set aside, and the views would overstate the open work.
Goal `tooling`; part of epic spec-9a3131.

## Where

The 18 items, all open at `41c7ffbd6`:
- **Dreams:** gap-4d1bff, bug-7257cb, bug-8c5d32, bug-b9be1d, bug-dee062, bug-c1649d.
- **Knowledge decay:** bug-429315, bug-dd5dca.
- **Affect:** reg-d76ab0.
- **Conductor tick:** gap-ebd656 (p1, `features`). Research note B6 says S06 and spec-a0403b supersede it.
- **LinUCB tuning:** find-3d8bd4, find-3f99f2, gap-47356e.
- **Roko's own channel layer:** gap-8d80d3 (p1), spec-743a7e (p1), gap-63055e, spec-b3ab28, spec-b4ba72. tldr/05
  decision 13 puts Roko behind Hermes and OpenClaw instead.

## Current state

Nothing has been parked. tldr/05 §6 recommends parking affect and dreams (decision 12) and Roko's own channel layer
(decision 13); the author has not confirmed either.

## Plan

The options:

1. **Park all 18 (recommended default).** Run `tools/work.py park <ids> --reason "tldr/05 §3: subsystem parked
   (decided <date>)"`. Parking is not closure: `unpark` restores an item, and parked defects stay searchable.
2. **Park 15, and keep the three knowledge-decay bugs open** (bug-8c5d32, bug-429315, bug-dd5dca). Under
   `roko serve` they damage the knowledge store, since every entry loses confidence about every 2.8 hours. S02.P1-6
   fixes all three at once.
3. **Keep all 18 open with a `hold`.** They stay visible in `STATUS.md` but drop out of `next`. This costs one edit
   per item and leaves the views noisy.

**Trade-off.** Option 1 gives the cleanest views, and parked defects can come back. Option 2 costs little and keeps a
data-corruption bug in view.

## Done when

- [ ] The author picks an option. Record the choice and the date here.
- [ ] The chosen items are parked (or held) with that reason, and `python3 tools/work.py check` reports 0 problems.
- [ ] This item's box is ticked in epic spec-9a3131.

## Notes

- This decision does not depend on bug-470de8. Turning the dream default off stops the spending either way.
- A decision has no `[[verify]]`. `work/parked/` is where parked items live.
