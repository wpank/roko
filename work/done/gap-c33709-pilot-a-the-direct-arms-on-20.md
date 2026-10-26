+++
id = "gap-c33709"
kind = "gap"
title = "Pilot A: the direct arms on 20 tasks (S09.E1a)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-09-29
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "d7c70c0d2"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S09-experiments.md (§6 E1a, §4.7 G0; checklist S09.E1a)"
anchors = ["benchmarks/viabilitybench/experiments/pilot_a.toml", "benchmarks/viabilitybench/reports/pilot_a/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-4723ff", "gap-9e7079", "gap-7ee7c2", "gap-b24517", "gap-33d54b", "gap-a8a160"], blocks = [], related = ["gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_a/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot_a"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T10:18:59Z"
commit = "d7c70c0d2"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-19T09:28:18Z"
forced = false
evidence = "d7c70c0d2 commits benchmarks/viabilitybench/reports/pilot_a/, from Pilot A's one paid run: `vb campaign --manifest pilot_a --allow-network --max-cost-usd 6 --transcripts` at 3220afb56 (clean tree), under env -i, 2026-10-19 09:51-10:06 UTC. Its units ran cheap-direct-s1, fd-api-s1, cheap-direct-s2, cheap-direct-s3 in daily-interleave order, one at a time, and each exited 0. vb estimate came first: cheap_direct $0.2893 a task at worst, fd_api $1.00; the campaign dry run gave 65 runs and $5.10 planned. Records 65 of 65: 64 completed, 1 aborted_cap (model_calls), 0 infra_error, 0 leak_suspected, 0 canary hits. Every agent and census ran under network none with sandbox-exec+net, and errors.jsonl is empty. VS: cheap_direct gpt-oss-120b 3/60 (l1 1/12, l2 2/12, l3-l5 0/12; 37 false greens among its 40 visible passes); fd_api gpt-5.4 5/5. Spend $1.2996 billed: cheap_direct $1.157858 on BL0 ($0.0193 a task, 0.32 x plan) and fd_api $0.141741 on BL8 ($0.0283 a task, 0.09 x plan). The ledger equals the metering proxy's meter to the micro-dollar. All 575 calls answered HTTP 200 and no row has an unknown cost, so bug-2a5566 had no effect. vb ledger report afterwards: BL0 $1.1579 of $14, BL8 $0.1417 of $8, pilot $1.2996 of $15, programme $1.3542 of $400, nothing reserved, OK. G0 (go-no-go.md, g0.json) is NO-GO. 6 checks pass: verifier CI 200/200 under secret sha256:1675cc951cf12ce3, the synthetic runaway (aborted_cap at 30 calls), canaries 0, l5 0.0 <= 0.3, and $/task for both arms. 1 fails: cheap l1 0.083 < 0.6, filed as gap-dd670b. 8 are not evaluated: the ledger against the provider exports (filed gap-1011ff), SC2 (filed gap-129862, with an 18-item blind worksheet) and Pilot B's checks. Verify: report.py --check reports/pilot_a gives 1 bundle, 0 problems. ~/.config/viabilitybench: the listing and the sha256 of keys and secret were unchanged before and after; the tripwire added empty keys.lock and secret.lock. Pilot B was prepared only: its pilot_b.toml dry run gives 145 runs, $6.08 planned billed and $12.15 at worst, inside BL0 and the pilot cap."
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

- 2026-10-02, claude-agent on `work/gap-c33709`: **ready for the paid run, waiting for the key file; $0 spent.**
  The provider key file `~/.config/viabilitybench/keys` does not exist yet. Will creates it, with
  `CEREBRAS_API_KEY=` and `OPENAI_API_KEY=` lines, mode 0600 in the 0700 directory; nobody looked for keys
  anywhere else. No results root exists yet (`~/.roko-bench/viability`), so no ledger row has been written.
- **The benchmark secret** is `/Users/will/.config/viabilitybench/secret`, fingerprint `sha256:1675cc951cf12ce3`
  (`driver/secret.py init`, 2026-10-02; mode 0600 in a 0700 directory, with a canary line). Pilot B must use this
  same file: each instance keeps one secret for the whole campaign (S09 §4.1).
- **Caps** (33efe9665, approved by the coordinator): the blocks' `max_cost_usd` are $5.40 (cheap-direct, $1.80 per
  seed) and $2.60 (fd-api), a hard $8.00 in all. Planned spend is unchanged at $5.10 ($3.60 on BL0, $1.50 on BL8).
  A seed's 20th cheap_direct task starts only if its first 19 averaged at most $0.0795, and fd_api's fifth only if
  its first four averaged at most $0.40 (1.33 × plan). c1fc06e4a adds `vb campaign --max-cost-usd`, which holds an
  experiment's billed spend under one cap in all; at these block caps it changes no unit's cap and states the bound.
- **Checked offline:**
  - `vb campaign --manifest experiments/pilot_a.toml --dry-run --max-cost-usd 8`, against the real secret and
    again under `env -i`: ok, 65 runs in 4 daily units, planned $5.10, worst case $8.00, no problems.
  - `experiments/test_pilot_a.py` at the new caps: 3 passed, including the 65-run offline rehearsal, whose units
    ran under `--max-cost-usd` 1.8, 1.8, 1.8 and 2.6.
  - **Network confinement:** all 65 rehearsal records say `provenance.network_policy = {network: none, sandbox:
    sandbox-exec+net}` and `vs.sandbox = sandbox-exec`, and `mini_loop.network_policy` sets the same for a paid
    run. `driver/test_sandbox_net.py`: 4 passed, none skipped.
  - Verifier CI under the real secret (`--families f1,f4 --levels 1-5 --seeds 10`, latents v1 and v2): 200/200
    cells green (f1 100/100, f4 100/100), covering all 20 pilot instances.
  - Synthetic runaway (a stub that never submits; cheap_direct on F1-l1-0001, seed 1, through the metering proxy):
    `aborted_cap` (`model_calls`) at 30 calls, in attempts of 12, 12 and 6 turns.
  - TLS from the venv's Python to both providers, with no credentials: api.cerebras.ai answers 403 `Not
    authenticated` and api.openai.com 401, so only the key is missing.
  - The scoring path, on the offline records: `vb report --bundle`, `report.py --check` (0 problems) and the
    `gates.py G0` page.
- **The paid run**, once `driver/secret.py keys` lists both names:

  ```bash
  cd /Users/will/dev/nunchi/roko/roko-work-gap-c33709 &&
  env -i HOME="$HOME" USER="$USER" LOGNAME="$USER" PATH="$PATH" TMPDIR="$TMPDIR" LANG=C.UTF-8 \
    benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/driver/vb.py campaign \
    --manifest benchmarks/viabilitybench/experiments/pilot_a.toml --allow-network --max-cost-usd 8 --transcripts
  ```

2026-10-15 (Will, 2026-10-15; tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md), decision 1: run the pilots. The key file ~/.config/viabilitybench/keys is Will's to create (CEREBRAS, OPENAI, MOONSHOT and ZAI lines copied from ~/.roko/.env; the command is in the coordinator session). Start when `python3 benchmarks/viabilitybench/driver/secret.py keys` reports the four names at mode 0600.

## Decision (2026-10-18, Will)

Unblocked: the key file ~/.config/viabilitybench/keys was created on 2026-10-18 at Will's request; Pilots A-C run on the $21 approved on 2026-10-02 (3301).

## Result (2026-10-19, claude-agent on `work/gap-c33709`)

**All 65 runs recorded; $1.2996 billed; G0 is NO-GO on the cheap ladder at level 1.** The bundle is
`benchmarks/viabilitybench/reports/pilot_a/`.

- **The run.**
  - At 3220afb56, with a clean tree, `secret.py keys` reported the four names at mode 0600, and `secret.py check`
    reported `sha256:1675cc951cf12ce3`.
  - `vb estimate`: cheap_direct $0.2893 a task at worst, fd_api $1.00. The campaign dry run gave 65 runs, $5.10
    planned and $8.00 in block caps.
  - The paid run: `vb campaign --manifest pilot_a --allow-network --max-cost-usd 6 --transcripts`, under
    `env -i` (USER, LOGNAME, PATH, TMPDIR, LANG; no HOME, so the bench resolved it from the password database),
    2026-10-19 09:51–10:06 UTC. Its units ran in daily-interleave order (cheap-direct-s1, fd-api-s1,
    cheap-direct-s2, cheap-direct-s3), one at a time, and each exited 0.
- **The outcomes.**
  - 65 of 65 records: 64 `completed` and 1 `aborted_cap` (`model_calls`, F1-l3-0001 seed 3). There were 0
    `infra_error`, 0 `leak_suspected` and 0 canary hits. Every agent and every census ran under network `none`
    with `sandbox-exec+net`. The proxy refused nothing, and `errors.jsonl` is empty.
  - VS: cheap_direct 3/60 (ℓ1 1/12, ℓ2 2/12, ℓ3–ℓ5 0/12), with 37 false greens among its 40 visible passes;
    fd_api 5/5.
- **The spend.**
  - cheap_direct $1.157858 on BL0, which is $0.0193 a task (0.32 × plan). fd_api $0.141741 on BL8, which is
    $0.0283 a task (0.09 × plan).
  - The ledger equals the metering proxy's meter to the micro-dollar. All 575 calls answered HTTP 200, and no
    row has an unknown cost.
  - `vb ledger report` afterwards: BL0 $1.1579 of $14, BL8 $0.1417 of $8, pilot $1.2996 of $15, programme
    $1.3542 of $400, nothing reserved.
- **bug-2a5566 had no effect here.** It prices an attempt at its reservation only when every call of that
  attempt failed, and none did.
- **G0** (`reports/pilot_a/go-no-go.md`, `g0.json`):
  - Pass: verifier CI 200/200 under the real secret (`verifier-ci.json`, trimmed from the 2.8 MB original);
    the synthetic runaway (`.runaway/`: offline stub through the proxy, `aborted_cap` at 30 calls in attempts of
    12/12/6); canaries 0; ℓ5 0.0 ≤ 0.3; $/task for both arms.
  - Fail: ℓ1 0.083 < 0.6. Filed **gap-dd670b**, which blocks gap-327242.
  - Not evaluated: the ledger against the provider exports (filed **gap-1011ff**: Will downloads the Cerebras
    and OpenAI usage CSVs for 2026-10-19 UTC), and SC2 (filed **gap-129862**: `sc2.toml` holds 18 blind items for
    a person; 2 more VS = 1 outputs come from Pilot B). Pilot B's own checks are not evaluated either.
- **`~/.config/viabilitybench`.** The listing and the sha256 of `keys` and `secret` were unchanged before and
  after. The tripwire left empty `keys.lock` and `secret.lock` files, and its chmods moved both ctimes.
- **Pilot B, prepared only.** The `pilot_b.toml` dry run with both optional blocks gives 145 runs, $6.075 planned
  billed and $12.15 at worst, and fits BL0 and the pilot cap. The header now records this.
