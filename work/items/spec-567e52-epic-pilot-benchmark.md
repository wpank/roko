+++
id = "spec-567e52"
kind = "spec"
title = "Epic: pilot benchmark"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "L"
subsystem = ["benchmarks/viabilitybench", "config/prices"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W10-benchmarks-proof.md (roadmap stage 1; §4, the quick proof)"
anchors = ["benchmarks/viabilitybench/driver/vb.py", "benchmarks/viabilitybench/analysis/report.py", "benchmarks/viabilitybench/reports/pilot/REPORT.md"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "bench"
links = { depends_on = ["dec-b78874", "gap-0580f7", "gap-2790c5", "gap-4723ff", "gap-9e7079", "gap-7ee7c2", "gap-28ebea", "gap-b24517", "gap-33d54b", "gap-e003ec", "gap-c4f364", "gap-b7ab99", "gap-a8a160", "gap-c33709", "gap-327242", "gap-d9e9fe", "gap-89f393", "gap-1cd676", "q-ab27d3", "gap-419298", "dec-39c781", "bug-993e7e"], blocks = [], related = ["bug-7d7200", "bug-35379d", "gap-644040", "bug-28becc", "bug-3a037b", "bug-a22228"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot/REPORT.md && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot"
+++

## Problem

Roko claims that cheap models inside Roko reach frontier quality at a fraction of the cost. No measurement
supports that claim yet:

- **No benchmark has hidden tests.** `benchmarks/` holds only `dev-audit/`, which measures development speed.
- **`roko bench` is not honest.** It scores the agent's own output, leaks the SWE-bench gold patch and shows
  simulated savings (S08 §3, D1–D8).
- **The field record allows no comparison.** It covers 158 verified tasks at $1.22 each, all on Sonnet 4.6 (W10).

## Why it matters

The author chose "cheaper at equal quality" as the first thing to prove (PLAN.md §1). This epic proves it as a
pilot:

- **Tasks:** 20 with hidden tests (families F1 and F4, levels ℓ1–ℓ5, two instances each), 3 seeds each.
- **Three arms on the same tasks:** `cheap_direct` (gpt-oss-120b in a bash-only loop), `roko_fixed` (gpt-oss-120b
  in Roko) and `fd_claude` (Claude Code on `claude-opus-5-5`, D2).
- **Per arm:** verified success (VS), cost per verified success ($/VS), pass^3 and false greens, with run ids.
- **Budget:** at most about $15 billed.

The whitepaper's evaluation section cites it (E1.12, gap-2aad7d), and S09's LOG1 campaign reuses the instrument.

## Where

- **New tree `benchmarks/viabilitybench/`** (name and path follow D4, dec-b78874): schemas, families, verifier CI,
  driver, arms, analysis, experiments, and committed pilot summaries in `reports/`.
- **New price snapshot:** `config/prices/2026-09-28.toml`. Raw results go to `$VB_RESULTS`, outside the repo.
- **Reused, not changed:** `scripts/dev_benchmark.py` (fail-closed network and cost admission); the `roko` CLI
  (`plan validate --strict --dag`, `plan run`, global `--model` and `--repo`); the `claude` flags in
  `ClaudeCliAgent::build_command`.

## Current state

Checked at `41c7ffbd6`: nothing of this exists. There is no `benchmarks/viabilitybench/` or `config/` directory,
every S08 and S09 row in `execution/checklist.json` is `todo`, and only gates D1–D3 are open.

- **Outside the epic:** the Roko arm needs bug-7d7200 (agents inherit roko's environment), whose fix `dc99a9e81`
  sits unmerged on the portal session's branch `fix/hermetic-child-env`. bug-28becc, bug-3a037b and bug-a22228 fix
  `roko bench`, which this driver replaces; they stay under `release` and are not children.
- **The arms differ from tldr/04,** which asks for Roko with a frontier executor and with escalation. This pilot
  follows D1 and W10: escalation was deleted with Runner-v2 (E5 brings it back), so `roko_fixed` uses one pinned
  model, and the confirmatory `roko_full` arm waits for gate G5.
- **U′ and R are swapped** in the checklist's acceptance for S09.E1b. S09 §4.1 is right: U′ = `modelUsage` ×
  snapshot (the headline), R = `total_cost_usd`.

## Plan

1. **Decide D4 and D42** (dec-b78874). Everything below waits for D4. The Claude Code runs also wait for D42.
2. **Lay the foundation.** First the tree, schemas and price snapshot (gap-0580f7), then the common library
   (gap-2790c5).
3. **Run three lanes in parallel:**
   - F1 (gap-4723ff);
   - F4 (gap-9e7079);
   - the direct-arm driver (gap-28ebea). It tests against a toy family, so it does not wait for F1 or F4.
4. **Finish the instrument.**
   - After F1 and F4: verifier CI (gap-7ee7c2).
   - After the driver, in parallel, since their files do not overlap:
     - the report (gap-b24517);
     - budget lines (gap-33d54b);
     - secret handling (gap-a8a160);
     - the metering proxy (gap-e003ec), optional for the pilot;
     - the Claude Code arm (gap-c4f364);
     - the Roko arm (gap-b7ab99), which also waits for bug-7d7200.
5. **Run Pilot A** (gap-c33709): the direct arms, 60 billed runs plus 5 `fd_api` runs. If the ladder check SC3
   fails, re-tune the levels before going on.
6. **Run Pilot B with seeds 2–3** (gap-327242): `roko_fixed` and `fd_claude`, 60 runs each.
7. **Publish one page** (gap-d9e9fe), with `vb report --pilot`.

Only steps 5 and 6 spend money, and both depend on the budget ledger. Every build item tests offline against stub
providers. This removes the checklist's circular `budget` gate (W10 rec 2).

## Done when

- [x] dec-b78874: Decide the benchmark's name and location (D4) and whether subscription terms allow scripted
      Claude Code runs (D42)
- [x] gap-0580f7: ViabilityBench tree, schemas and price snapshot (S08.T1)
- [x] gap-2790c5: ViabilityBench common library: pristine repos, knobs, seeding, AST checks and canaries (S08.T2)
- [x] gap-4723ff: ViabilityBench family F1: generator, truth suite and gaming detector (S08.T3)
- [x] gap-9e7079: ViabilityBench family F4: store-state truth suite and partial-failure injection (S08.T4)
- [ ] gap-7ee7c2: ViabilityBench verifier CI: reference, stub and gaming solutions, determinism and leak checks
      (S08.T5)
- [x] gap-28ebea: ViabilityBench driver for the direct arm (S08.T6)
- [ ] gap-b24517: ViabilityBench report: verified success, cost per verified success, pass^k and false greens
      with run ids (S08.T7)
- [ ] gap-33d54b: ViabilityBench budget lines and caps in the run ledger (S09.E2)
- [ ] gap-e003ec: ViabilityBench metering and fault proxy (S08.T13)
- [ ] gap-c4f364: ViabilityBench Claude Code arm with an isolated config (S08.T12)
- [ ] gap-b7ab99: ViabilityBench Roko arm with the model pinned and checked on every attempt (S08.T11)
- [ ] gap-a8a160: Keep VB_SECRET in a driver-only file and prove agents cannot read it
- [ ] gap-c33709: Pilot A: the direct arms on 20 tasks (S09.E1a)
- [ ] gap-327242: Pilot B plus seeds 2–3: the Roko and Claude Code arms on the same tasks (S09.E1b)
- [ ] gap-d9e9fe: vb report --pilot: a one-page pilot result with confidence intervals and run ids
- [x] gap-89f393: Plan-level slice: fixture features that need whole multi-task plans (added 2026-09-29)
- [ ] gap-1cd676: Run the plan-level slice: Roko with the ladder against Claude Code (added 2026-09-29)
- [ ] q-ab27d3: Should the fd_claude_lite arm run Claude Sonnet 5.5, which is priced the same as Sonnet 5?
- [x] gap-419298: S09 v1.2: register the plan-level slice as an exploratory experiment
- [ ] dec-39c781: Confirm decisions D28–D36 before the pre-registration lock
- [ ] bug-993e7e: ViabilityBench astcheck counts the __pycache__ an honest agent's test run writes as an added test file
- [ ] The epic's `[[verify]]` command passes: the pilot page is committed, and `report.py --check` accepts its
      bundle.

## Notes

- **Budget (the author must confirm).** S09 caps BL0 at $10, and five `fd_api` runs take about $1.5 of BL8. Seeds
  2–3 for `roko_fixed` add about $3 (W10 rec 7), so BL0's cap rises to $14 and an experiment-level cap holds the
  pilot to $15. S09's caps then total $394: under its $400 limit, with at least $100 unallocated.
- **pytest** runs the Python verify commands but is not installed for the system `python3` (3.12.8) today. It is a
  dev-only dependency; the benchmark code stays stdlib-only. `work.py close` gives each verify command 60 s.
- **The author's part:** the SC2 spot check (20 outputs, about 2 hours) and the Cerebras and OpenAI usage exports.
  `fd_claude` shares the author's subscription with the development agents, so it runs off-hours.
- **Framing:** "a pilot on two Python families: descriptive, not pre-registered". At 20 × 3 only the cost ratio is
  likely to resolve; VS differences under about 0.3 will not (W10). A null harness effect is possible, because F4's
  visible check passes dry runs in every arm.
- **Out of scope:** the mini-swe-agent and Codex runners (S08.T12), families F2–F8, the SWE-bench slice,
  `roko_full`, and the Rust fixes to `roko bench` (S08.T17).
- **Hot files:** none, and no cargo. The `bench` lane takes 1–2 agents at a time.
- **Decided 2026-09-29 (Will):** D4 is ViabilityBench at `benchmarks/viabilitybench/`, with pilot summaries committed under `reports/`. D42: the Claude Code arm runs on the subscription. The benchmark uses a pinned project venv, created by gap-0580f7. The evaluation covers single tasks plus a plan-level slice (gap-89f393, gap-1cd676).
- **Still open (not accepted on 2026-09-29):** raising budget line BL0 from $10 to $14 for seeds 2-3 (the pilot stays capped at $15).
