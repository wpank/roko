+++
id = "gap-f3944c"
kind = "gap"
title = "Cerebras answers HTTP 402 payment_required to every call since 2026-10-20 15:17Z, so every gpt-oss-120b run (Pilot B's roko_fixed arm, Pilot C, LOG1) is blocked until the account has credit"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "15335e2f9"
source = "gap-327242 worker (claude-agent), 2026-10-20/21: Pilot B"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/experiments/pilot_b.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = ["gap-327242"], related = ["gap-327242", "bug-c6318e", "gap-7ce3ca", "gap-aa60f2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "ls benchmarks/viabilitybench/reports/pilot_b/ | grep -q '^roko-fixed-'"
+++

## Problem

Since 2026-10-20 15:17:44Z, Cerebras has answered every chat completion with HTTP 402:

```
{"message": "Payment required to access this resource. Visit your billing tab.", "type": "payment_required_error",
 "param": "quota", "code": "payment_required"}
```

The answer is the same with the ViabilityBench key file (`~/.config/viabilitybench/keys`), whose Cerebras line was
copied from `~/.roko/.env` on 2026-10-18, so the account behind both is probably out of credit or past a spend limit.
It was still 402 at 18:06, 23:28 and after the UTC day boundary (02:05 and 02:43 on 2026-10-21). A request naming a
model that does not exist still gets 404 `model_not_found`, so `vb capture-errors` cannot show whether the account
works.

## Why it matters

- Every gpt-oss-120b run is blocked: Pilot B's `roko_fixed` arm (gap-327242), its optional `provider-fault` block,
  Pilot C (gap-aa60f2), LOG1 and any roko run that routes to gpt-oss-120b.
- Pilot B's three roko units ran in their order on 2026-10-20/21. Their tasks hit the 402s and ended `infra_error`
  (bug-c6318e), and the units are in `~/.roko-bench/viability/PILOT-B.abandoned/`. The `fd_claude` arm finished
  all 60 runs.

## Where

- Cerebras's billing tab, for the account whose key `~/.roko/.env` and `~/.config/viabilitybench/keys` hold.
  Only Will can add credit.
- Pilot B's evidence: `~/.roko-bench/viability/PILOT-B.abandoned/roko-fixed-s1-2/proxy.jsonl` (the first 402 at
  15:17:44Z; the last 200 at 15:15:43Z).

## Current state

Blocked on the account, not the code. Every Pilot B roko run on Cerebras has been moved aside, so the campaign will
redo `roko-fixed-s1`, `-s2` and `-s3` in its order (gap-7ce3ca).

## Plan

1. Will adds Cerebras credit, or raises the spend limit.
2. Rerun `vb campaign --manifest benchmarks/viabilitybench/experiments/pilot_b.toml --allow-network
   --max-cost-usd 10.70 --transcripts` under `env -i` (gap-327242's progress has the command). It redoes roko seeds
   1-3 and skips the finished fd_claude units. `--include provider-fault` adds the optional fault rows.
3. Rebuild `reports/pilot_b/` and its G0 page.

## Done when

- [ ] Pilot B's bundle holds roko_fixed runs: the `[[verify]]` command.

## Notes

- Found by gap-327242's worker on 2026-10-20 during Pilot B.
- No key was read or printed. The 402 bodies come from the metering proxy's log, which never holds a key.
- Checked 2026-10-26 (gap-f3944c worker, claude-agent): `vb capture-errors --provider cerebras --allow-network
  --max-cost-usd 0.01` (driver/error_capture.py, gap-db8dab), one `nonexistent-model` probe through the metering
  proxy, fixture written outside the repo so `driver/testdata/provider_errors/cerebras.json` is untouched. Cerebras
  answered HTTP 404 `application/json` at 2026-10-26T10:34:22Z: `{"message":"Model does not exist or you do not
  have access to it.","type":"not_found_error","param":"model","code":"model_not_found"}` — not 402. Ledger:
  `billed_usd: 0.0`, `cost_source: not_billed`; spend $0.00.
- That 404 is not evidence the block has lifted: the body is byte-for-byte identical to the pre-block capture of
  2026-10-18 (`driver/testdata/provider_errors/cerebras.json`, `captured_at` 2026-10-18T18:40:35Z), and this item's
  own Problem section already recorded the same 404 `model_not_found` during the confirmed-402 window on
  2026-10-21. Cerebras validates the model name before, or independently of, the account's billing state, so a
  nonexistent-model probe reads the same whether the account is blocked or has credit; it cannot show whether a
  real (valid-model) call still gets 402, which is what this item needs to close. Left open. No key was read,
  printed or copied, and no header or request/response body beyond what is quoted above was examined.
