+++
id = "gap-64ac11"
kind = "gap"
title = "Rebuild the pilot page (reports/pilot/) once Pilot B's roko_fixed arm has its 60 runs, so roko_fixed, its cost ratio and the harness effect stop being pending"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "gap-d9e9fe (vb report --pilot), 2026-10-26"
anchors = ["benchmarks/viabilitybench/reports/pilot/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-327242"], blocks = [], related = ["gap-d9e9fe", "gap-f3944c", "bug-ec7f84", "dec-3d5714"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot/REPORT.md && ! grep -q 'roko_fixed (pending)' benchmarks/viabilitybench/reports/pilot/REPORT.md && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot"
+++

## Problem

The pilot page, `benchmarks/viabilitybench/reports/pilot/REPORT.md` (gap-d9e9fe), was built on 2026-10-26 from
partial data:

- **Two of the three arms have runs.** `cheap_direct` comes from PILOT-A-RECAL and `fd_claude` from PILOT-B, each
  20 tasks × 3 seeds.
- **`roko_fixed` has no usable run.** Cerebras answers HTTP 402 (gap-f3944c), so Pilot B's `roko-fixed` block
  (seeds 1–3, 60 runs) is pending.
- **What the page therefore shows as pending:** the `roko_fixed` row, the `roko_fixed ÷ fd_claude` cost ratio, and
  the harness effect (`roko_fixed − cheap_direct`).

## Why it matters

The page is the first proof point (W10 rec 8). Its question is whether gpt-oss-120b inside Roko is cheaper than
Claude Code at equal quality, which needs `roko_fixed`. Today the page answers only for the direct loop, and the
answer is "cheaper per verified success, not at equal quality".

## Where

- `benchmarks/viabilitybench/reports/pilot/`, rebuilt with `vb report --pilot`. The exact command is in the page's
  Provenance section. Drop the note about `roko_fixed`'s 402s once it no longer applies.
- `reports/pilot_b/`, rebuilt first by gap-327242.

## Current state

Checked on 2026-10-26: `reports/pilot/REPORT.md` lists `roko_fixed (pending)`, with 0 of 60 planned runs.

## Plan

1. When gap-327242 lands `roko_fixed`'s 60 runs, and with them a rebuilt `reports/pilot_b/`, regenerate the page
   into an empty `reports/pilot/` at B = 10,000, from a committed analysis tree.
2. If the optional provider-fault block has run by then, fix bug-ec7f84 first. Without that fix the merge refuses
   PILOT-B's repeated (arm, task, seed) keys.
3. Commit the page, and update the whitepaper's citation (E1.12, gap-2aad7d) if it quotes numbers.

## Done when

- [ ] The page lists `roko_fixed` with numbers and intervals, and its cost ratio against `fd_claude`, with nothing
      pending in the three-arm comparison.
- [ ] The `[[verify]]` command passes.

## Notes

- Filed by gap-d9e9fe's worker on 2026-10-26, which closed gap-d9e9fe on the partial data the lead allowed.
- If dec-3d5714 changes the bootstrap before the rebuild, the rebuild uses the new method and says so.
