# G0 go/no-go: NO-GO

Experiments PILOT-A-RECAL; computed 2026-10-19T12:17:41Z at 2775a1b523daf2b2dfc5e1d2c0077a1ea33dd5f1; thresholds sha256:0bc1857cf2bc9ef4ed80f4a9c3635ea13e86f21797bcac7b5c284fbe477d531a. 5 pass, 1 fail, 9 not evaluated. A check without its evidence is not evaluated, never passed.

| check | value | threshold | verdict | run ids |
|---|---|---|---|---|
| Verifier CI, F1 and F4 | f1 100/100, f4 100/100 | 100% of cells green | **pass** | - |
| Ledger against the providers' usage exports |  | every drift within ±5% | **not evaluated** | cheap-direct-1 |
| Synthetic runaway stopped | run_id runaway-1, reason model_calls, calls 30 | every run stopped by the runaway detector (model_calls, identical_calls, usd) within 30 calls | **pass** | runaway-1 |
| Canary hits | 0 | at most 0 | **pass** | cheap-direct-1 |
| SC2 spot check | - | at least 20 outputs (10 of each census label), 95% agreeing | **not evaluated** | - |
| Ladder: cheap_direct gpt-oss-120b at level 1 | 0.5 | VS ≥ 0.6 | **fail** | cheap-direct-1 |
| Ladder: cheap_direct gpt-oss-120b at level 5 | 0.0 | VS ≤ 0.3 | **pass** | cheap-direct-1 |
| Ladder: fd_claude at level 5 | - | VS ≥ 0.4 | **not evaluated** | - |
| Measured $/task, cheap_direct | 0.017615 | ≤ 2 × the plan's $0.06 | **pass** | cheap-direct-1 |
| Measured $/task, fd_api | - | ≤ 2 × the plan's $0.3 | **not evaluated** | - |
| Measured $/task, roko_fixed | - | ≤ 2 × the plan's $0.075 | **not evaluated** | - |
| U′ and R for every fd_claude run | - | 100% captured; gap reported | **not evaluated** | - |
| Injected fault rates | - | within ±2 points of each profile's p, over ≥ 20 | **not evaluated** | - |
| Every roko_fixed record ingested | - | all planned runs recorded; no model mismatch kept | **not evaluated** | - |
| Direct-loop choice (S08 decision 3) | - | one of mini-loop, mini-swe-agent, recorded | **not evaluated** | - |

## Details

- **Verifier CI, F1 and F4** (pass): secret sha256:1675cc951cf12ce3
- **Ledger against the providers' usage exports** (not evaluated): no reconcile JSON for cerebras (--reconcile)
- **Synthetic runaway stopped** (pass): the synthetic runaway's runs
- **Canary hits** (pass): over 60 runs
- **SC2 spot check** (not evaluated): no hand-filled spot check (--sc2, vb.sc2/1 with [[item]] tables)
- **Ladder: cheap_direct gpt-oss-120b at level 1** (fail): 12 runs on 4 tasks
- **Ladder: cheap_direct gpt-oss-120b at level 5** (pass): 12 runs on 4 tasks
- **Ladder: fd_claude at level 5** (not evaluated): no kept run at that level
- **Measured $/task, cheap_direct** (pass): 0.29 × the plan over 60 runs
- **Measured $/task, fd_api** (not evaluated): no fd_api run
- **Measured $/task, roko_fixed** (not evaluated): no roko_fixed run
- **U′ and R for every fd_claude run** (not evaluated): no fd_claude run
- **Injected fault rates** (not evaluated): no provider_fault run with its proxy log in these runs
- **Every roko_fixed record ingested** (not evaluated): no roko_fixed run
- **Direct-loop choice (S08 decision 3)** (not evaluated): not recorded: [direct_loop] choice in the --sc2 file

## Evidence

- runs: benchmarks/viabilitybench/reports/pilot_a_recal
- verifier_ci: benchmarks/viabilitybench/reports/pilot_a_recal/verifier-ci.json
- runaway: benchmarks/viabilitybench/reports/pilot_a_recal/.runaway
- budget: benchmarks/viabilitybench/experiments/budget.toml
