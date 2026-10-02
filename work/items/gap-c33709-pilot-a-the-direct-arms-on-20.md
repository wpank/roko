+++
id = "gap-c33709"
kind = "gap"
title = "Pilot A: the direct arms on 20 tasks (S09.E1a)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S09-experiments.md (§6 E1a, §4.7 G0; checklist S09.E1a)"
anchors = ["benchmarks/viabilitybench/experiments/pilot_a.toml", "benchmarks/viabilitybench/reports/pilot_a/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-4723ff", "gap-9e7079", "gap-7ee7c2", "gap-b24517", "gap-33d54b", "gap-a8a160"], blocks = [], related = ["gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_a/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot_a"
+++

## Problem

Nobody has measured a cheap model working alone on tasks with hidden tests. The instrument has not met real model
output either: not the verifiers, the ledger, the caps or the canaries.

## Why it matters

- **It is the first arm.** gpt-oss-120b in the direct loop, on 20 tasks × 3 seeds, is the cheap-model-alone arm of
  the three-arm comparison.
- **It checks the instrument** through G0's direct-arm checks (S09 §4.7).
- **It calibrates the ladder** before Pilot B spends money.
- **It tests billed cost accounting.** Five billed `fd_api` runs (gpt-5.4) check it on a billed frontier model.

## Where

All new, and the paths follow D4:

- **`benchmarks/viabilitybench/experiments/pilot_a.toml`:** the manifest.
- **`reports/pilot_a/`:** the committed summary bundle, with records, `metrics.json`, the ledger and a go/no-go
  page. It holds no transcripts and no archives.
- **Raw results** go to `$VB_RESULTS/PILOT-A/`.

## Current state

Checked at `41c7ffbd6`: nothing can run yet, and no money has been spent.

## Plan

1. **The manifest.**
   - Stream `pilot`: F1 and F4, at levels ℓ1–ℓ5, two instances each, 20 tasks in all.
   - `cheap_direct` on gpt-oss-120b, seeds 1–3: 60 runs on budget line BL0.
   - `fd_api` on gpt-5.4: 5 runs on BL8.
2. **Estimate, then run.** `vb estimate` gives the planned spend; then `vb run --allow-network --max-cost-usd …`.
3. **Score.**
   - Run the census, `vb report` and `vb ledger report`.
   - Reconcile the ledger with the Cerebras and OpenAI usage exports. The author downloads these.
4. **Record every G0 direct-arm check with its value:**
   - verifier CI at 100%;
   - a synthetic runaway killed;
   - 0 canary hits;
   - SC2: a person reads 20 outputs, and at least 19 must agree with their labels;
   - ladder sanity: VS ≥ 0.6 at ℓ1 and ≤ 0.3 at ℓ5;
   - cost per task at most 2× the plan.
5. **Calibrate if needed.** If the ladder check (SC3) fails, re-tune the levels before Pilot B (S08 §4.4), and file
   an item for it.
6. **Commit** the bundle.

## Done when

- [ ] All 65 runs are recorded, or accounted for as `infra_error` or `aborted_cap`.
- [ ] Spend stays within BL0's share plus the 5 BL8 runs.
- [ ] Every G0 direct-arm check is on the go/no-go page, with its value.
- [ ] The `[[verify]]` command passes.

## Notes

- **This item spends money.** Get the author's go-ahead before starting.
- **One runner at a time,** from the author's machine. Never run it through roko.
- **No hot files.**

## Progress

- 2026-10-02, claude-agent on `work/gap-c33709`: **blocked before any paid call; $0 spent.** The driver finds no
  provider key file (`secret.py keys`: `~/.config/viabilitybench/keys`, no such file; no `VB_KEY_FILE`) and no
  benchmark secret (`~/.config/viabilitybench/secret`). Per the run rules nobody looked for keys elsewhere. No
  results root exists yet (`~/.roko-bench/viability`), so no ledger row has been written.
- Done offline, at 6f85e5efc:
  - `vb campaign --manifest experiments/pilot_a.toml --dry-run`: 65 runs (4 daily units), planned $5.10 (BL0
    $3.60, BL8 $1.50), worst case $10.20 (blocks' caps $7.20 + $3.00; worst task $0.2893 and $1.00).
  - `experiments/test_pilot_a.py`: 3 passed, including the 65-run offline rehearsal. Every rehearsal record says
    `provenance.network_policy = {network: none, sandbox: sandbox-exec+net}`, which `mini_loop.network_policy`
    sets whether or not the provider is offline. `driver/test_sandbox_net.py`: 4 passed, none skipped.
  - Verifier CI, `--families f1,f4 --levels 1-5 --seeds 10` (latents v1 and v2, throwaway secret
    sha256:8ae87d2143c8b7ae): 200/200 cells green (f1 100/100, f4 100/100) in 401 s.
  - Synthetic runaway (a stub that never submits; cheap_direct on F1-l1-0001, seed 1, through the metering
    proxy): `aborted_cap` (`model_calls`) at 30 calls, attempts of 12, 12 and 6 turns.
  - The scoring path, rehearsed on the offline records: `vb report --bundle`, `report.py --check` (0 problems) and
    the `gates.py G0` page.
- c1fc06e4a: `vb campaign --max-cost-usd` holds an experiment's billed spend under one hard cap (the coordinator's
  $8 for Pilot A): each billed unit gets the smaller of its share of its block's cap and what the experiment's books
  leave. Lowering the manifest's block caps to $8 instead would cut a unit short once its tasks average more than
  1.33 × plan, and a cut unit leaves the bundle failing `report.py --check`.
- To unblock (Will): create the key file, mode 0600 in a 0700 directory, with `CEREBRAS_API_KEY=` and
  `OPENAI_API_KEY=` lines (keys or projects used only by the pilot keep the usage-export reconciliation clean), and
  the benchmark secret (`driver/secret.py init`). The paid run is then `vb campaign --manifest
  benchmarks/viabilitybench/experiments/pilot_a.toml --allow-network --max-cost-usd 8 --transcripts`.
