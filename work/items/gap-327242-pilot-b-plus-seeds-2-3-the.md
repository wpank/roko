+++
id = "gap-327242"
kind = "gap"
title = "Pilot B plus seeds 2–3: the Roko and Claude Code arms on the same tasks (S09.E1b)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S09-experiments.md (§6 E1b; checklist S09.E1b); workstreams/assessment/W10-benchmarks-proof.md (rec 7)"
anchors = ["benchmarks/viabilitybench/experiments/pilot_b.toml", "benchmarks/viabilitybench/reports/pilot_b/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-c4f364", "gap-b7ab99", "gap-c33709"], blocks = [], related = ["gap-e003ec", "dec-b78874"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_b/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot_b"
+++

## Problem

Neither the Roko arm nor the Claude Code arm has any runs yet. S09's Pilot B also runs each of them with one seed
only, which leaves pass^3 out of the three-arm comparison.

## Why it matters

- **It completes the comparison the author wants first:** `cheap_direct` (from Pilot A), `roko_fixed` and
  `fd_claude`, on the same 20 tasks × 3 seeds.
- **Seeds 2–3 are cheap.** W10 rec 7 adds them for about $3 billed and 40 more subscription runs.
- **It covers the rest of G0.**

## Where

All new, and the paths follow D4:

- **`benchmarks/viabilitybench/experiments/pilot_b.toml`:** the manifest.
- **`reports/pilot_b/`:** the committed summary bundle.
- **Raw results** go to `$VB_RESULTS/PILOT-B/`.

## Current state

Checked at `41c7ffbd6`: nothing can run yet. Two answers are still missing:

- D42, whether the subscription terms allow scripted runs (dec-b78874);
- whether BL0's cap may rise to $14 (gap-33d54b).

## Plan

1. **The manifest.**
   - `roko_fixed` on gpt-oss-120b, seeds 1–3: 60 runs, billed to BL0.
   - `fd_claude` on `claude-opus-5-5`, seeds 1–3: 60 runs on the subscription. $0 is billed, and the API-equivalent
     cost is logged.
2. **Order.** Interleave the arms in daily blocks (S09 §4.1), and run `fd_claude` off-hours.
3. **Record the remaining G0 checks:**
   - U′ and R captured for 100% of `fd_claude` runs, with the gap between them measured. Use S09 §4.1's
     definitions; the checklist's acceptance for this row swaps the two.
   - `fd_claude` reaches VS ≥ 0.4 at ℓ5.
   - Every `roko_fixed` record ingests, with no model mismatch outside the `infra_error` runs.
   - 0 canary hits.
4. **Optional runs.** S09's other Pilot B rows, mini-swe-agent × 20 and `provider_fault` × 5 (gap-e003ec), run only
   if their code exists and the budget allows.
5. **Commit** the bundle.

## Done when

- [ ] All 120 runs are recorded or accounted for.
- [ ] Billed spend stays within BL0 and within the pilot's $15 cap.
- [ ] The `[[verify]]` command passes.

## Notes

- **Blocked until:**
  - D42 is answered yes, or a fallback is chosen;
  - the author gives the go-ahead to spend.
- **Subscription limits.** The weekly limits are shared with the development agents.
- **Infra errors.** Exclude them and count them. Treat the first `roko_fixed` runs as a shakedown.
- **No hot files.**
