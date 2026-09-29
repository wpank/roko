+++
id = "dec-b78874"
kind = "decision"
title = "Decide the benchmark's name and location (D4) and whether subscription terms allow scripted Claude Code runs (D42)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "70820a74c"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/DECISIONS.md (D4, D42); workstreams/assessment/W10-benchmarks-proof.md (rec 1)"
anchors = ["benchmarks/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-1cd8d3"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "Will (decided 2026-09-29)"
evidence = "Will decided 2026-09-29. D4: ViabilityBench at benchmarks/viabilitybench/ (tracked source); each pilot's small summary bundle (records, metrics, ledger, report page; no transcripts) is committed under benchmarks/viabilitybench/reports/; raw runs stay in ~/.roko-bench/viability. D42: yes, the subscription permits scripted claude -p runs, so the Claude Code arm runs on the subscription. Also decided: a pinned project venv for the benchmark's tests, and an evaluation that covers single tasks plus a small plan-level slice (gap-89f393, gap-1cd676)."
+++

## Problem

Two open decisions hold up the pilot benchmark:

- **D4, the name and location.** Every S08 build row waits on the checklist gate `decision:D4`, which is still
  closed (only D1–D3 are open). The default in `DECISIONS.md` is ViabilityBench at `benchmarks/viabilitybench/`,
  tracked, source only, with results outside the repo.
- **D42, the subscription terms.** D1 (Plan A) runs the Claude Code arm on the author's subscription as scripted
  `claude -p` loops. Anthropic's terms bar automated access "except where we otherwise explicitly permit it". Only
  the author can check this.

## Why it matters

D4 gates every item in epic spec-567e52, and the speclint slice (E8.4, gap-1cd8d3), which lives under the
benchmark tree. D42 gates the Claude Code arm (gap-c4f364) and Pilot B (gap-327242); without that arm the pilot
cannot show "cheaper at equal quality".

## Where

- `tmp/cybernetic-harness/DECISIONS.md`: D4 and D42.
- S08 §4.1 (name options) and §9.1–2 (location).
- `tmp/cybernetic-harness/execution/checklist.json`: the gate `decision:D4`.

## Current state

Checked at `41c7ffbd6`: `benchmarks/` holds only `dev-audit/`, and no benchmark tree exists. Both decisions are
unanswered.

## Plan

1. **D4, the name.** Default ViabilityBench (suite id `vb`, schemas `vb.*/1`); S08 §4.1 lists Steersman and
   HomeoBench as alternatives.
2. **D4, the location.** Default `benchmarks/viabilitybench/`, tracked, source only; raw results in `$VB_RESULTS`
   (default `~/.roko-bench/viability`).
3. **D4, one addition.** Commit each pilot's small summary bundle (records, metrics, ledger and report page; no
   transcripts or archives) under `benchmarks/viabilitybench/reports/`? Default yes: the whitepaper cites them and
   the pilot items' verify commands read them. The trade-off (S08 §9.2): agents that can read the repo can read the
   generators and hidden suites too, so workdirs stay outside the repo and canaries detect reads.
4. **D42.** If scripted runs are not allowed, billed Claude Code costs about $0.4–1.2 per task (S09 §3): about
   $24–72 for the 60 `fd_claude` runs, over the $15 pilot budget. The fallbacks are S09's Plan B (the billed
   `fd_api` arm) or `fd_claude` on seed 1 only.
5. **Record the answers** in `DECISIONS.md` and run
   `python3 tmp/cybernetic-harness/tools/checklist.py gate-open decision:D4`. If the name or path changes, update
   the E12 items' paths, which all follow D4.

## Done when

- [ ] D4 is answered in `DECISIONS.md`: the name, the path and the `reports/` question. The checklist gate is
      open.
- [ ] D42 is answered. If the answer is no, gap-c4f364 and gap-327242 say which fallback they use.

## Notes

- **Also for the author:** the pilot budget change in epic spec-567e52's Notes. BL0's cap goes from $10 to $14,
  and the whole pilot is capped at $15.
- **Waits on:** only the author.
