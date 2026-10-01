+++
id = "bug-9bffe2"
kind = "bug"
title = "roko status --cfactor prints avg_cost=$0.0000 for bench efficiency events whose cost is unknown"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/commands/util"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "286c5e53a"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-960ab1 at b482d7830)"
anchors = ["crates/roko-cli/src/commands/util.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-a445eb", "bug-9a6799"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cfactor_shows_unknown_cost_as_unknown' crates/roko-cli/src/ && cargo test -p roko-cli cfactor_shows_unknown_cost_as_unknown"
+++

## Problem

`roko status --cfactor` prints each group's `avg_cost` and `p95_cost` (`crates/roko-cli/src/commands/util.rs:932`). Bench efficiency events whose cost is unknown (bug-a445eb) are averaged as $0, so a group made of them shows `avg_cost=$0.0000`.

## Why it matters

One settled record per attempt (epic spec-b7303f): an unknown cost shown as $0 reads as free. p3.

## Where

The cfactor aggregation in `commands/util.rs`.

## Plan

1. Exclude unknown costs from the averages. Print `unknown` when a group has none known, and the count of unknown ones otherwise.
2. Add `cfactor_shows_unknown_cost_as_unknown`.

## Done when

- [ ] `--cfactor` never shows $0 for a cost nobody measured.
- [ ] The `[[verify]]` command passes.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-0320da` at `722d486f4`; cargo verification deferred to the batch check. The known-cost rule is now `AgentEfficiencyEvent::has_known_cost()` in roko-learn, shared with the dashboard. Not covered: `roko learn`'s per-role table (`commands/learn.rs:2740`) still averages unknown costs as $0 through `compute_role_profiles`.
