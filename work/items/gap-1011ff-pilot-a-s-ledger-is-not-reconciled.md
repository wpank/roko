+++
id = "gap-1011ff"
kind = "gap"
title = "Pilot A's ledger is not reconciled with the Cerebras and OpenAI usage exports, so G0's ±5% ledger check is not evaluated (needs Will's downloads)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "gap-c33709 worker (claude-agent), 2026-10-19: Pilot A's G0 page, benchmarks/viabilitybench/reports/pilot_a/go-no-go.md"
discovered_from = "gap-c33709"
anchors = ["benchmarks/viabilitybench/reports/pilot_a/", "benchmarks/viabilitybench/driver/ledger.py::reconcile"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot_a/reconcile-cerebras.json && test -f benchmarks/viabilitybench/reports/pilot_a/reconcile-openai.json && python3 -c \"import json, sys; checks = {c['id']: c['verdict'] for c in json.load(open('benchmarks/viabilitybench/reports/pilot_a/g0.json'))['checks']}; sys.exit(0 if checks.get('ledger_reconcile') == 'pass' else 1)\""
+++

## Problem

G0's check "ledger within ±5% of the Cerebras and OpenAI exports" (S09 §4.7) is not evaluated on Pilot A's go/no-go
page (`benchmarks/viabilitybench/reports/pilot_a/go-no-go.md`), because nobody has downloaded the providers' usage
exports. `vb ledger reconcile` compares the ledger with an export, but only the account owner can download one.

What the ledger holds for Pilot A (experiment PILOT-A, all on 2026-10-19 UTC, priced from `prices-2026-09-28`):

| provider | model | calls | UTC window | input tokens (cached incl.) | cached | output | ledger $ |
|---|---|---|---|---|---|---|---|
| cerebras | gpt-oss-120b | 557 | 09:51:57–10:06:29 | 2,128,414 | 1,549,056 | 550,551 | 1.157858 |
| openai | gpt-5.4 (gpt-5.4-2026-03-05) | 18 | 09:55:49–09:56:52 | 37,409 | 9,216 | 4,597 | 0.141741 |

The ledger matches the metering proxy's own meter to the micro-dollar. Both price the same reported usage from the
same snapshot, though, so only the providers' exports check the price snapshot and the usage the providers report.

## Why it matters

- **G0 needs it:** it cannot be GO without this check (gap-c33709).
- **It is the only external check of billed cost accounting** (S08 SC5), and five billed gpt-5.4 runs exist for it.
- **One open question rides on it.** Cerebras reported 73% of gpt-oss-120b's prompt tokens as cached (1,549,056 of
  2,128,414). The snapshot prices a cached token at the input rate ($0.35/M). If Cerebras bills cached tokens for
  less, the ledger overstates Cerebras spend by up to $0.54 of its $1.16, and the snapshot needs a new dated file.

## Where

- `benchmarks/viabilitybench/driver/ledger.py` (`vb ledger reconcile`, `read_export`, `EXPORT_COLUMNS`).
- `benchmarks/viabilitybench/analysis/gates.py G0 --reconcile FILE` (one per provider).
- The bundle: `benchmarks/viabilitybench/reports/pilot_a/`.

## Current state

- **No export has been downloaded.** The key file (`~/.config/viabilitybench/keys`, created 2026-10-18) holds keys
  copied from `~/.roko/.env`, so other roko work may have used the same keys on 2026-10-19.
- **The comparison is per UTC day** (and per model with `--by-model`), not per hour.

## Plan

1. **Will downloads the two exports, in USD, as CSV:**
   - Cerebras: the usage export for 2026-10-19 (UTC) from the Cerebras Cloud console's usage/analytics page, for
     model gpt-oss-120b, limited to the API key whose name is `CEREBRAS_API_KEY` in the key file if the console
     can filter by key. The columns needed are date, model, input (prompt) tokens, cached tokens if shown, output
     tokens, and cost.
   - OpenAI: platform.openai.com → Usage → Export, for 2026-10-19 (UTC), model gpt-5.4, limited to the project or
     API key of `OPENAI_API_KEY`, with the same columns.
   - If the console cannot limit the export to the key, and the key served other traffic that day, also export an
     hourly view of 09:00–11:00 UTC. The day totals would then include traffic that is not the benchmark's.
2. **Reconcile each export** from the repository root, saving the results in the bundle:
   ```bash
   PY=benchmarks/viabilitybench/.venv/bin/python
   $PY benchmarks/viabilitybench/driver/vb.py ledger reconcile --provider cerebras --csv CEREBRAS.csv \
       --experiment PILOT-A --since 2026-10-19 --until 2026-10-19 --by-model --json \
       > benchmarks/viabilitybench/reports/pilot_a/reconcile-cerebras.json
   $PY benchmarks/viabilitybench/driver/vb.py ledger reconcile --provider openai --csv OPENAI.csv \
       --experiment PILOT-A --since 2026-10-19 --until 2026-10-19 --by-model --json \
       > benchmarks/viabilitybench/reports/pilot_a/reconcile-openai.json
   ```
   If a header is not recognized, map it with `--column FIELD=HEADER`.
3. **Recompute the G0 page:** the command in gap-c33709's closing note, plus `--reconcile` with both files. If the
   Cerebras cost drifts past 5% while the tokens agree, check the cached-token rate and file the new snapshot.

## Done when

- [ ] Both reconcile JSON files are in the bundle.
- [ ] Every comparison is within ±5%, or each drift is explained and filed.
- [ ] The G0 page's ledger check passes: the `[[verify]]` command.

## Notes

- **The downloads need Will's provider accounts.** No agent reads the key file or logs in to a provider console.
- **Commit no raw export**, only the reconcile JSON. An export can name the account or keys.
