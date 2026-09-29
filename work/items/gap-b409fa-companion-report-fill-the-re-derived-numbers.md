+++
id = "gap-b409fa"
kind = "gap"
title = "Companion report: fill the re-derived numbers into the E10 draft and fix the flagged source errors"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "48d35a67f"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wave-1 reports)"
anchors = ["tmp/cybernetic-harness/companion-audit/E10-DRAFT.md", "tmp/cybernetic-harness/companion-audit/00-README.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '\\[\\[E1:' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md && grep -q '91b4745f8' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Companion E10-DRAFT (tmp, wk-companion-fin): all 364 E1 markers filled from E1-REDERIVATION at 91b4745f8, exceptions listed; source errors fixed: README S1 row moved from the round-1 figure (195 adjudicated) to the current estimator on 469 rows, '2 of 13 core-path', FALSE-GREENS lag 0.06-0.38 s, V1-V3 anchors @91b4745f8 (V2 main.rs:4365), 131 FS anchors to CLAUDE.md@91b4745f8 (15 stamped @393a72002); 10 scripts copied to companion-audit/telemetry/scripts. Verify passes in MAIN; E2/E3 placeholders left for raters."
+++

## Problem

`tmp/cybernetic-harness/companion-audit/E10-DRAFT.md` (gap-652d05) marks every number `[[E1: …]]`. `tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md` (gap-cdd5f4) has now
re-derived 101 numbers at `91b4745f8`, covering every marker. The re-derivation also flagged these source errors:
- the README's S1 row (195 rows were adjudicated, not 469);
- "13 core-path claims effective", which should be 2 of 13;
- the FALSE-GREENS lag range, which should read 0.06–0.38 s;
- anchors pointing at a file added after the tag (103 of them map to `CLAUDE.md` at the tag);
- the V2 SIGTERM anchor, which is `crates/roko-cli/src/main.rs:4365@91b4745f8`.

## Why it matters

The companion report is the first publication candidate. Its numbers must be the re-derived ones. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/companion-audit/E10-DRAFT.md`, `tmp/cybernetic-harness/companion-audit/00-README.md` and the companion's source notes; the numbers come from
`tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md`.

## Current state

The draft has about 360 E1 markers; E2 and E3 still wait for human raters.

## Plan

1. **Numbers:** replace each `[[E1: …]]` with its re-derived value, stamped with the tag.
2. **Source errors:** fix each one listed above in the companion files.
3. **Scripts:** move the drift and E1 helper scripts out of the session scratchpad into
   `tmp/cybernetic-harness/companion-audit/telemetry/scripts/`. The re-derivation's report says which ones.
4. **E2 and E3:** leave the `[[E2:…]]` and `[[E3:…]]` placeholders in place.

## Done when

- [ ] No `[[E1:` markers remain, and the draft cites the tag.
- [ ] The source errors are fixed.
- [ ] The `[[verify]]` command passes.

## Notes

- These files are untracked, so edit them in place in the main checkout. Edit only this item's anchored files.
- Keep every `[[RESULT …]]` slot, and use the marker grammar and the `observational` claim type in
  `tmp/cybernetic-harness/paper/00-README.md`.
- The working title is "A Cybernetic Harness for Spec'd Agent Work: Frontier Models Plan, Cheap Models Execute"; the
  thesis is golden path plus cybernetics. Use the status tags in `docs/whitepaper/data/mechanisms.toml`.
