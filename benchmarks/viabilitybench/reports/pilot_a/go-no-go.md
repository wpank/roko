# G0 go/no-go: NO-GO

Experiments PILOT-A; computed 2026-10-19T10:11:07Z at 3220afb56a5bff4654a41cb0186b1b210becbebf; thresholds sha256:0bc1857cf2bc9ef4ed80f4a9c3635ea13e86f21797bcac7b5c284fbe477d531a. 6 pass, 1 fail, 8 not evaluated. A check without its evidence is not evaluated, never passed.

| check | value | threshold | verdict | run ids |
|---|---|---|---|---|
| Verifier CI, F1 and F4 | f1 100/100, f4 100/100 | 100% of cells green | **pass** | - |
| Ledger against the providers' usage exports |  | every drift within ±5% | **not evaluated** | cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1, fd-api-s1-1 |
| Synthetic runaway stopped | run_id runaway-1, reason model_calls, calls 30 | every run stopped by the runaway detector (model_calls, identical_calls, usd) within 30 calls | **pass** | runaway-1 |
| Canary hits | 0 | at most 0 | **pass** | cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1, fd-api-s1-1 |
| SC2 spot check | 0/0 | at least 20 outputs (10 of each census label), 95% agreeing | **not evaluated** | cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1, fd-api-s1-1 |
| Ladder: cheap_direct gpt-oss-120b at level 1 | 0.0833 | VS ≥ 0.6 | **fail** | cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1 |
| Ladder: cheap_direct gpt-oss-120b at level 5 | 0.0 | VS ≤ 0.3 | **pass** | cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1 |
| Ladder: fd_claude at level 5 | - | VS ≥ 0.4 | **not evaluated** | - |
| Measured $/task, cheap_direct | 0.019298 | ≤ 2 × the plan's $0.06 | **pass** | cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1 |
| Measured $/task, fd_api | 0.028348 | ≤ 2 × the plan's $0.3 | **pass** | fd-api-s1-1 |
| Measured $/task, roko_fixed | - | ≤ 2 × the plan's $0.075 | **not evaluated** | - |
| U′ and R for every fd_claude run | - | 100% captured; gap reported | **not evaluated** | - |
| Injected fault rates | - | within ±2 points of each profile's p, over ≥ 20 | **not evaluated** | - |
| Every roko_fixed record ingested | - | all planned runs recorded; no model mismatch kept | **not evaluated** | - |
| Direct-loop choice (S08 decision 3) | - | one of mini-loop, mini-swe-agent, recorded | **not evaluated** | - |

## Details

- **Verifier CI, F1 and F4** (pass): secret sha256:1675cc951cf12ce3
- **Ledger against the providers' usage exports** (not evaluated): no reconcile JSON for cerebras, openai (--reconcile)
- **Synthetic runaway stopped** (pass): the synthetic runaway's runs
- **Canary hits** (pass): over 65 runs
- **SC2 spot check** (not evaluated): items without a run record or a 0/1 human_label: fd-api-s1-1/F1-l1-0002.s1, cheap-direct-s3-1/F4-l3-0001.s3, fd-api-s1-1/F1-l5-0001.s1, cheap-direct-s2-1/F4-l1-0002.s2, cheap-direct-s3-1/F4-l4-0002.s3
- **Ladder: cheap_direct gpt-oss-120b at level 1** (fail): 12 runs on 4 tasks
- **Ladder: cheap_direct gpt-oss-120b at level 5** (pass): 12 runs on 4 tasks
- **Ladder: fd_claude at level 5** (not evaluated): no kept run at that level
- **Measured $/task, cheap_direct** (pass): 0.32 × the plan over 60 runs
- **Measured $/task, fd_api** (pass): 0.09 × the plan over 5 runs
- **Measured $/task, roko_fixed** (not evaluated): no roko_fixed run
- **U′ and R for every fd_claude run** (not evaluated): no fd_claude run
- **Injected fault rates** (not evaluated): no provider_fault run with its proxy log in these runs
- **Every roko_fixed record ingested** (not evaluated): no roko_fixed run
- **Direct-loop choice (S08 decision 3)** (not evaluated): not recorded: [direct_loop] choice in the --sc2 file

## Evidence

- runs: benchmarks/viabilitybench/reports/pilot_a
- verifier_ci: benchmarks/viabilitybench/reports/pilot_a/verifier-ci.json
- sc2: benchmarks/viabilitybench/reports/pilot_a/sc2.toml
- runaway: benchmarks/viabilitybench/reports/pilot_a/.runaway
- budget: benchmarks/viabilitybench/experiments/budget.toml
