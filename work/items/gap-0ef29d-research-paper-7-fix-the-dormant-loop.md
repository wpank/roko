+++
id = "gap-0ef29d"
kind = "gap"
title = "Research paper §7: fix the dormant-loop claim in §7.3, align with the thesis, and trim"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b2c14cd4a"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wave-1 reports)"
anchors = ["tmp/cybernetic-harness/paper/sections/07-results-regulation.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/07-results-regulation.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Research paper §7 (tmp, edited in place by wk-rp-s7): §7.3 corrected to 17 inert loops (15 dormant, 2 masked) at the companion's tag, with 6 of the 10 S03 census targets companion-dormant; claim C7.18 names the unit of each loop count (17 inert, 16 Runner-v2 ids, 10 census targets); aligned with the golden-path thesis. Verify: paperlint --budget 1.2 --check-identifiers clean at 15:29; CLAIMS-EVIDENCE regenerated (417 claims, --check up to date)."
+++

## Problem

- §7.3 calls all ten census loops "companion-dormant", but only 6 of them are (gap-cdd5f4's re-derivation).
- The loop counts 17, 16 and 10 measure different things, and §7 must say which one it uses.
- §7 is about 2.3× its budget.

## Why it matters

§7 carries the regulation results for M1–M4. It must not overstate dormancy. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/07-results-regulation.md`. Sources: `tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md` and the tldr
research note B5 (2 of 16 loops fully wired at `98ee1418f`).

## Current state

The results slots exist, and the §7.3 claim is wrong.

## Plan

1. **§7.3:** correct the claim, and name the unit of each loop count.
2. **Framing:** align §7 with the thesis, keeping the M1–M4 slots.
3. **Trim:** cut to at most 1.2× budget.

## Done when

- [ ] §7.3 is corrected, and the file is within 1.2× budget.
- [ ] The `[[verify]]` command passes.

## Notes

- These files are untracked, so edit them in place in the main checkout. Edit only this item's anchored files.
- Keep every `[[RESULT …]]` slot, and use the marker grammar and the `observational` claim type in
  `tmp/cybernetic-harness/paper/00-README.md`.
- The working title is "A Cybernetic Harness for Spec'd Agent Work: Frontier Models Plan, Cheap Models Execute"; the
  thesis is golden path plus cybernetics. Use the status tags in `docs/whitepaper/data/mechanisms.toml`.
