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
updated = 2026-10-18
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S09-experiments.md (§6 E1b; checklist S09.E1b); workstreams/assessment/W10-benchmarks-proof.md (rec 7)"
anchors = ["benchmarks/viabilitybench/experiments/pilot_b.toml", "benchmarks/viabilitybench/reports/pilot_b/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-c4f364", "gap-b7ab99", "gap-c33709", "gap-dd670b"], blocks = [], related = ["gap-e003ec", "dec-b78874", "gap-129862"], supersedes = [], duplicate_of = "" }

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

2026-10-15 (Will, 2026-10-15; tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md), decision 6: dec-20744e accepted (option a), so Pilot B's subscription arms may run once Pilot A has passed its G0 checks.

## Decision (2026-10-18, Will)

Unblocked: the key file ~/.config/viabilitybench/keys was created on 2026-10-18 at Will's request; subscription use was accepted on 2026-10-15 (decision 6).

## Progress

- 2026-10-19, claude-agent (gap-c33709's worker): **prepared only, $0 spent on Pilot B.**
  - **Pilot A has run.** It billed $1.2996: $1.1579 on BL0 and $0.1417 on BL8. Its G0 page is NO-GO on the
    cheap ladder at level 1, where gpt-oss-120b solved 1 of 12 runs. gap-c33709's plan re-tunes the ladder before
    Pilot B, so this item now depends on gap-dd670b, which re-knobs F1 and F4. Pilot B can still run first if Will
    decides so; its roko_fixed arm runs the same 20 instances, whose level 1-2 tasks are about to change.
  - **The manifest is unchanged apart from its comments.** The header records the numbers below, and the
    mini-swe block's comment no longer says that its arm is missing.
  - **The estimate,** from `vb campaign --manifest pilot_b --dry-run --include provider-fault --include mini-swe`
    against the real results root:
    - **Size:** 145 runs, $6.075 planned billed, $12.15 at worst. That fits BL0 ($1.1579 held of $14) and the
      pilot cap ($1.2996 held of $15), with no problems found.
    - **roko_fixed:** per `vb estimate`, $0.434 a task at worst over 60 runs. The block's max_cost_usd holds it
      to $9.
    - **fd_claude:** $5 a task API-equivalent at worst, held to $120 API-equivalent; it bills $0.
  - **Prerequisites** before a paid run:
    - a built `target/debug/roko` in the checkout that runs the campaign (the main checkout has one; the
      gap-c33709 worktree does not);
    - mini-swe-agent's venv at `~/.local/share/mini-swe-agent/venv` for the optional `mini-swe` block (absent);
    - the fd_claude arm on Will's subscription, run off-hours, when he decides.
  - **SC2 needs 2 of this pilot's VS = 1 outputs.** Pilot A's worksheet holds only 8 VS = 1 outputs, so gap-129862
    takes the last 2 from here.
