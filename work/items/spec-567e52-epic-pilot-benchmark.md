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
links = { depends_on = ["dec-b78874", "gap-0580f7", "gap-2790c5", "gap-4723ff", "gap-9e7079", "gap-7ee7c2", "gap-28ebea", "gap-b24517", "gap-33d54b", "gap-e003ec", "gap-c4f364", "gap-b7ab99", "gap-a8a160", "gap-c33709", "gap-327242", "gap-d9e9fe", "gap-89f393", "gap-1cd676", "q-ab27d3", "gap-419298", "dec-39c781", "bug-993e7e", "gap-204848", "gap-455aef", "gap-db9a26", "gap-8c3752", "gap-04e8e2", "bug-3c1c4a", "bug-2930a8", "bug-979a06", "gap-308373", "gap-6e1381", "gap-154f93", "bug-f62293", "gap-f253cf", "gap-e90ebd", "dec-1089ec", "bug-b70d40", "gap-60654d", "bug-a49003", "gap-4667a4", "gap-0bd49a", "bug-32eb77", "gap-bc0640", "bug-09fac4", "bug-c30764", "gap-dad97b", "bug-d34a29", "gap-806e37", "gap-15bb83", "gap-44632a", "gap-a6abd5", "gap-29ac83", "gap-4e8795", "gap-8bdf5e", "gap-98516b", "bug-cd5000", "bug-a05c53"], blocks = [], related = ["bug-7d7200", "bug-35379d", "gap-644040", "bug-28becc", "bug-3a037b", "bug-a22228"], supersedes = [], duplicate_of = "" }

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
- [x] gap-7ee7c2: ViabilityBench verifier CI: reference, stub and gaming solutions, determinism and leak checks
      (S08.T5)
- [x] gap-28ebea: ViabilityBench driver for the direct arm (S08.T6)
- [x] gap-b24517: ViabilityBench report: verified success, cost per verified success, pass^k and false greens
      with run ids (S08.T7)
- [x] gap-33d54b: ViabilityBench budget lines and caps in the run ledger (S09.E2)
- [x] gap-e003ec: ViabilityBench metering and fault proxy (S08.T13)
- [x] gap-c4f364: ViabilityBench Claude Code arm with an isolated config (S08.T12)
- [x] gap-b7ab99: ViabilityBench Roko arm with the model pinned and checked on every attempt (S08.T11)
- [x] gap-a8a160: Keep VB_SECRET in a driver-only file and prove agents cannot read it
- [ ] gap-c33709: Pilot A: the direct arms on 20 tasks (S09.E1a)
- [ ] gap-327242: Pilot B plus seeds 2–3: the Roko and Claude Code arms on the same tasks (S09.E1b)
- [ ] gap-d9e9fe: vb report --pilot: a one-page pilot result with confidence intervals and run ids
- [x] gap-89f393: Plan-level slice: fixture features that need whole multi-task plans (added 2026-09-29)
- [ ] gap-1cd676: Run the plan-level slice: Roko with the ladder against Claude Code (added 2026-09-29)
- [ ] q-ab27d3: Should the fd_claude_lite arm run Claude Sonnet 5.5, which is priced the same as Sonnet 5?
- [x] gap-419298: S09 v1.2: register the plan-level slice as an exploratory experiment
- [ ] dec-39c781: Confirm decisions D28–D36 before the pre-registration lock
- [x] bug-993e7e: ViabilityBench astcheck counts the __pycache__ an honest agent's test run writes as an added test file
- [x] gap-204848: ViabilityBench schemas can't hold a plan-slice run record without a placeholder ladder, or a PL task
- [x] gap-455aef: S09 §4.9 says the plan-slice arms share no visible checks, but both get the same base repo and its tests
- [ ] gap-db9a26: The plan-slice hidden suites were written by the arms' own model family and need a cross-family review
- [x] gap-8c3752: The census runs hidden.py, which executes agent code, without the sandbox agents get
- [x] gap-04e8e2: ViabilityBench plan-slice records carry no queue waits or per-class costs, so the report prints them as not recorded
- [x] bug-3c1c4a: ViabilityBench metrics group runs by arm only, so an arm that runs two models in one experiment is pooled
- [x] bug-2930a8: vb run cannot materialize any F1 or F4 instance: materialize.py expects the .vb/ layout both families dropped
- [x] bug-979a06: The ViabilityBench driver holds provider API keys in its own environment, where same-uid agents can read them
- [x] gap-308373: A same-uid agent can read the ViabilityBench secret file silently, and nothing detects it
- [x] gap-6e1381: ViabilityBench docs are stale: README status lines, the gap-a8a160 pointer, records' s01_run_dir and S08 decision 8
- [ ] gap-154f93: Run the live fd_claude probe and replace the invented modelUsage fixture with its saved output
- [x] bug-f62293: A killed Claude Code session's partial cost is labelled cli_usage, because the schema rejects estimated with $0 billed
- [x] gap-f253cf: fd_claude keeps WebSearch and WebFetch, so once the repo is public an agent can fetch the hidden suites
- [x] gap-e90ebd: Wiring the metering proxy into vb run must configure each task and keep network admission for a loopback proxy URL
- [ ] dec-1089ec: Raise ViabilityBench budget line BL0's cap from $10 to $14 for Pilot B's seeds 2–3
- [x] bug-b70d40: The ViabilityBench provider client ignores a top-level cached_tokens and prices that input at the full rate
- [x] gap-60654d: ViabilityBench: enforce token caps through the proxy, fill the meter cross-check, and bundle proxy.jsonl
- [x] bug-a49003: A killed subscription session's ledger row still says cli_usage; it needs an estimated cost and a subscription marker
- [x] gap-4667a4: S08 still describes VB_SECRET as an environment variable at lines 106 and 302
- [ ] gap-0bd49a: Agent shells in every ViabilityBench arm can reach the network, so an agent can fetch the public repo's hidden suites
- [x] bug-32eb77: Operator credentials that aren't arm keys, such as ANTHROPIC_API_KEY or GITHUB_TOKEN, stay readable by agents through the driver's environment
- [x] gap-bc0640: Only one vb run can use a secret file at a time, and S09's run schedule doesn't account for it
- [x] bug-09fac4: The metering proxy stamps whole seconds, so an attempt that ends in the same second as the one before gets no usage
- [x] bug-c30764: The metering proxy forwards the call that crosses input_token_cap, so a task can overshoot its input cap by one call
- [x] gap-dad97b: run_roko.py must read bug-31438d's new record fields: turns_unknown, model_reported, substitution, attempt_key and helper rows
- [x] bug-d34a29: The metering proxy's input-token bound (request bytes + 256) assumes text-only requests; an image by URL can cost more
- [x] gap-806e37: The Roko arm doesn't enforce S08's 150K per-attempt input cap, so S08-SC6 can't hold for that arm
- [x] gap-15bb83: vb run always uses the proxy's clean profile, and the other disturbance hooks aren't built, so H6 has no disturbance mechanism
- [x] gap-44632a: No CI workflow runs the ViabilityBench verifier CI in benchmarks/viabilitybench/ci/
- [x] gap-a6abd5: S08 §5.2's heading still gives the task manifest as DIR/.vb/task.json
- [ ] gap-29ac83: The benchmark has no sandbox on Linux, so records there say sandbox: none
- [x] gap-4e8795: flaky_verify needs a visible-verify wrapper, and no arm has one
- [x] gap-8bdf5e: model_swap needs served-model checks, in records and run_roko, that accept a declared swap
- [x] gap-98516b: F1 and F4 render latent v1 only, so the convention_flip disturbance is refused
- [x] bug-cd5000: ViabilityBench's gate_verdict enum and metrics.py's NOT_PASSED don't list already_satisfied
- [x] bug-a05c53: ViabilityBench's Roko arm fails plan validate --strict on PLAN_041 since gap-dbf2a6, so every Roko-arm task ends infra_error
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
