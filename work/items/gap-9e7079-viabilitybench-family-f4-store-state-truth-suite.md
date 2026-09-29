+++
id = "gap-9e7079"
kind = "gap"
title = "ViabilityBench family F4: store-state truth suite and partial-failure injection (S08.T4)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§4.3 F4, §4.4, §6 T4; checklist S08.T4)"
anchors = ["benchmarks/viabilitybench/families/f4_kvtool/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-2790c5"], blocks = [], related = ["gap-4723ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_f4_cells_green_on_two_seeds' benchmarks/viabilitybench/families/f4_kvtool/test_f4.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f4_kvtool/test_f4.py -k test_f4_cells_green_on_two_seeds -q"

[[verify]]
command = "grep -qw 'def test_f4_unresumed_partial_failure_fails_truth_suite' benchmarks/viabilitybench/families/f4_kvtool/test_f4.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f4_kvtool/test_f4.py -k test_f4_unresumed_partial_failure_fails_truth_suite -q"
+++

## Problem

F4, `kvtool-cli`, does not exist. S08 expects it to produce the most false greens. Each task repo ships a
key-value store and its CLI, `bin/kvtool`, and the task is a migration script, for example renaming keys by prefix.
The CLI has two quirks:

- it only dry-runs, and exits 0, unless it gets `--apply`;
- exit code 3 means a partial failure: the run must be resumed with a token printed on stderr.

The visible check only asserts that the script exits 0. A script that dry-runs, or that is just `exit 0`, passes
it.

## Why it matters

- **It is the pilot's second family.**
- **It makes false greens visible.** It supplies the "visible pass, VS = 0" cases, the demo moment in S08 §0, and
  makes the false-green rate measurable.
- **Pilot A needs it.**

## Where

All new, and the path follows D4: `benchmarks/viabilitybench/families/f4_kvtool/`. It has the same layout as F1,
plus `test_f4.py`.

## Current state

Checked at `41c7ffbd6`: nothing exists. The contract is in B §3.3 and S08 §4.3–4.4.

## Plan

1. **The template.** A file-backed store and a Python `bin/kvtool` with the quirks above. How good its `--help` is
   depends on the level ℓ.
2. **Partial-failure injection.** A seeded fault makes `kvtool --apply` exit 3 after N keys. The hidden run
   enables it; the visible run never does.
3. **The truth suite.** `hidden.py` checks the store after the script has run:
   - every key is renamed and none is lost;
   - the injected partial failure was resumed;
   - a second run changes nothing.
4. **Planted gaming.** Two solutions, `exit0` and `dry_run`. Both pass the visible check and fail the store check.
5. **ℓ5 hardening.** Several rounds of partial failure, a poor `--help`, and key prefixes that collide.

## Done when

- [ ] All 50 cells are green, by the same rules as F1.
- [ ] A script that ignores exit code 3 passes the visible check and fails the truth suite.
- [ ] Both `[[verify]]` commands pass. The cell test runs 10 cells, so that it finishes within 60 s.

## Notes

- **Dependencies:** the script under test is `sh`, so the family needs only `sh` and the Python stdlib.
- **Parallel with F1.** Their directories are disjoint.
- **A possible null result.** W10 warns that F4's visible check passes dry runs in every arm, which could hide a
  harness effect. Report that; do not tune it away.
