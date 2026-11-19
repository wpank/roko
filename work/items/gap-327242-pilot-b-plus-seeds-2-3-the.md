+++
id = "gap-327242"
kind = "gap"
title = "Pilot B plus seeds 2–3: the Roko and Claude Code arms on the same tasks (S09.E1b)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-09-29
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S09-experiments.md (§6 E1b; checklist S09.E1b); workstreams/assessment/W10-benchmarks-proof.md (rec 7)"
anchors = ["benchmarks/viabilitybench/experiments/pilot_b.toml", "benchmarks/viabilitybench/reports/pilot_b/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-c4f364", "gap-b7ab99", "gap-c33709", "gap-dd670b", "gap-f3944c"], blocks = [], related = ["gap-e003ec", "dec-b78874", "gap-129862", "bug-291992", "bug-c6318e", "gap-7ce3ca", "gap-c7c462", "bug-bb6b73", "find-697944", "find-496f7c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "report.py --check only validates bundle well-formedness, and it already passes ('1 bundle, 0 problems' per the item's own note) against the current partial/NO-GO bundle, so it is not a completeness check and can pass long before all 120 runs exist. Add a check that metrics.json shows >=60 non-infra_error roko_fixed records (and >=60 for fd_claude) before accepting, alongside the existing test -f + report.py --check."
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

## Decision (2026-10-19, Will)

Run fd_claude overnight (about 23:00–07:00 local) once gap-dd670b's re-tune lands; the cheap roko_fixed arm runs alongside, interleaved as S09 §4.1 says.

## Progress (2026-10-20/21, claude-agent on `work/gap-327242`)

**fd_claude: 60 of 60 runs done. roko_fixed: none of its 60 runs usable yet.** Since 2026-10-20 15:17Z Cerebras has
answered every call with HTTP 402 payment_required (gap-f3944c). The item stays open.

- **How it ran.**
  - The roko binary: `target/debug/roko`, built at c962955af (local main, clean) from an APFS copy of roko-pr-1017's
    target, through heavy.sh.
  - The bench commits: b3bd1a7b0 for roko seed 1, 967340b5d for every later unit. Clean trees.
  - The command: `vb campaign --manifest experiments/pilot_b.toml --allow-network --max-cost-usd 10.70
    --transcripts --units 1`, one unit at a time, under `env -i` (USER, LOGNAME, PATH, TMPDIR, LANG).
  - Audits off as asked. `[roko] audit = false` makes the runner pass `plan run --no-audit` (f5c2d6d9d), and the
    manifest's header and block notes say so. Every Roko arm but roko_full now does the same (gap-c7c462).
- **Fixes the runs needed.** Each has its own item, closed on this branch:
  - bug-291992: 10 of the 20 pilot specs run past the focused tier's 350 words, so `plan validate --strict` failed
    on PLAN_TIER_SIZE alone.
  - bug-c6318e: a task the provider refused for the account (401/402) was recorded as Roko's own failure; it is now
    `infra_error`.
  - gap-7ce3ca: the campaign now redoes a finished unit whose run was moved to `PILOT-B.abandoned/`.
- **fd_claude, seeds 1-3, all inside the 23:00-07:00 window.** Units: fd-claude-s1-1 (2026-10-20 23:00-23:26),
  s2-1 (2026-10-21 02:11-02:40) and s3-1 (02:46-03:17).
  - Outcomes: 60/60 `completed`, VS 60/60 (1.0 at every level, pass^3 1.0). 0 false greens, 0 `infra_error`,
    0 `leak_suspected`, 0 canary hits.
  - Model: Claude Code 2.1.282, served by claude-opus-5-5 only; modelUsage names no other model.
  - Effort: a median of 6 turns and 51 s a task, 80 min of sessions in all.
  - Network: the egress proxy admitted 727 requests to api.anthropic.com and refused 200, all to Claude Code's
    telemetry host, http-intake.logs.us5.datadoghq.com.
  - No usage limit was hit.
- **G0's remaining checks.**
  - **U′ and R for every fd_claude run: pass.** S09 §4.1's definitions: U′ is modelUsage's tokens × the snapshot,
    with 1-hour cache writes (`api_equiv_usd`); R is `total_cost_usd` (`vendor_usd`). Both were captured for 60 of
    60 runs. ΣU′ = ΣR = $15.1947, and the largest per-run |U′ − R|/R is 2.4e-16, float rounding.
  - **fd_claude VS ≥ 0.4 at ℓ5: pass,** 1.0 over 12 runs on 4 tasks. That is above S08's frontier band from ℓ3 up,
    filed as find-496f7c.
  - **Every roko_fixed record ingested: not evaluated.** No usable roko_fixed run exists. roko-fixed-s1-2's 20
    records did ingest with no model mismatch, but that unit is spoiled (below).
  - **0 canary hits** over the 120 runs on the page.
- **roko_fixed, in order.** Every run is now in `~/.roko-bench/viability/PILOT-B.abandoned/`.
  - roko-fixed-s1-1 (2026-10-20 16:56): stopped after 7 tasks, 4 of them failing validation (bug-291992); $0.1463.
  - roko-fixed-s1-2 (17:10-17:31): 20 records, $0.7768. The first 11 tasks are real: VS 3/11 (F1-l1-0001,
    F1-l2-0001, F4-l1-0002), 4 false greens, no model mismatch, $0.07 a task. The last 9 met the 402s and were
    recorded `failed`, from before bug-c6318e.
  - roko-fixed-s1-3 (18:06) and roko-fixed-s2-1 (23:28): 402 on every call; each stopped after 9 and 6 tasks,
    all `infra_error`.
  - roko-fixed-s2-2 (02:05, past the UTC day boundary), roko-fixed-s3-1 (02:43) and roko-fixed-s1-4 (03:20): each
    ran in its campaign slot, and all 20 tasks of each ended `infra_error` on 402, $0.
  - Moving them aside means the next campaign run redoes roko-fixed-s1, -s2 and -s3 in order, skipping the finished
    fd_claude units.
- **Optional blocks.**
  - mini-swe: not run; there is no venv at `~/.local/share/mini-swe-agent/venv`.
  - provider-fault: not run; it is roko_fixed on Cerebras.
- **Spend.**
  - Billed: $0.923105 on BL0, all Cerebras, all before the 402s.
  - The ledger counts $1.7911 for PILOT-B. Two stopped tasks' unknown costs are bound at their $0.434 reservations,
    though every call they made was refused (bug-bb6b73).
  - `vb ledger report` afterwards: BL0 $4.0058 of $14, the pilot $4.1476 of $15 ($10.85 left), the programme $4.2022
    of $400, nothing reserved, OK.
  - fd_claude: $0 billed, $15.1947 API-equivalent.
- **Bundle.** `reports/pilot_b/` holds fd_claude's three runs, `metrics.json`, `ledger-report.json`, `go-no-go.md`
  and `g0.json`.
  - The G0 page is computed over this bundle and `reports/pilot_a_recal/` (the same suite, f1-1.2.0 and f4-1.2.0),
    with A-RECAL's verifier CI and runaway.
  - It is NO-GO: 7 pass, 1 fail and 7 not evaluated. The fail is cheap ℓ1 at 0.50 against 0.6 (gap-dd670b,
    gap-0ff34c).
  - `report.py --check reports/pilot_b`: 1 bundle, 0 problems.
- **Home directories.** The listings and sha256 of `~/.config/viabilitybench` (4 entries, mode 0600) and `~/.roko`
  (132 entries) were identical before (2026-10-20 14:07Z) and after (2026-10-21 01:22Z). No key was read or
  printed. `--credential-fingerprint` was not used, since it reads the keychain token.
- **What remains.**
  1. roko_fixed seeds 1-3: 60 runs, about $1.5-2.5 billed at seed 1's $0.07 a task, once Cerebras has credit
     (gap-f3944c). Rerun the campaign command above; it starts at roko-fixed-s1.
  2. Optionally `--include provider-fault`.
  3. Rebuild `reports/pilot_b/` and its G0 page.
  4. gap-129862's last two VS = 1 outputs for SC2 can come from fd_claude's runs now.

Checked 2026-10-27: 2026-10-26: unchanged from the item's own 2026-10-20/21 progress note. roko_fixed (60 of 120 runs) remains blocked on gap-f3944c (Cerebras 402s); rerun the campaign from roko-fixed-s1 once credit is restored.
