# Refiner cost sample (task 3236)

Run `refine-3236-20261018b` of experiment `REFINE-COST`, started 2026-10-18T18:24:15Z, on budget line BL15. The spec refiner R-v1 (`specops/refine.py`, at most 2 additive rounds) ran with the live adapter `driver/refine_live.py` (prompt `refine-live-1`, sha256 `b18d64ed9fcd…`) on `gpt-oss-120b` (cerebras, live, through the metering proxy), priced from `prices-2026-09-28`. Harness `8431b7f65087`; config `sha256:43a2e6289a2a…`.

- **Tasks:** 20 of 20, band C or D under `sq-3` (static): 10 from fixtures, 10 from plans, drawn with seed 3236 (the active plans under `plans/` and speclint's fixtures).
- **Rounds:** 40 kept over 20 task(s); 40 model call(s), 40 answered by the model; 0 task(s) never answered. Why refinement stopped: rounds_exhausted 20.
- **Cost a task** (the ledger's, at the reported usage): mean $0.0027, median $0.0026, max $0.0050; 0 unknown. Total $0.0546 API-equivalent and $0.0546 billed on BL15 (20 ledger row(s)); the proxy metered $0.0546 for the same calls.
- **Tokens:** 29,357 input, 18,432 cached, 50,547 output (32,441 of it reasoning).
- **Score change** (`sq-3`, static): mean 45.3 -> 79.0, mean change +33.6, largest +46.6; bands before C 16 D 4, after A 8 B 12.
- **Hard fails** (S07 §4.2; they block whatever the score): 2 task(s) had one before, 11 after; added: HF2 on 1 task(s), HF4 on 10 task(s). A round is kept when the score does not drop (`refine`'s guard), even one that adds a hard fail.
- **Per-task budget:** $0.02 (S07 §4.3), held before every call and reserved on the ledger for each task; at most 4096 output tokens a call.

within the $0.02 cap: yes

| # | From | Task | Score before | Score after | Hard fails | Rounds | Stopped | Calls | Cost |
|---|---|---|---|---|---|---|---|---|---|
| 1 | plans | `plans/demos/demo-multistage/tasks.toml` STAGE-1A | 49.7 C | 74.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0033 |
| 2 | plans | `plans/demos/demo-parallel-integration/tasks.toml` PAR-RIGHT | 44.7 C | 79.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0034 |
| 3 | plans | `plans/portal-programme/05-portal-foundation/tasks.toml` T02 | 53.1 C | 88.1 A | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0027 |
| 4 | plans | `plans/demos/demo-incident-tabletop/tasks.toml` INC-4 | 43.4 C | 74.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0031 |
| 5 | plans | `plans/demos/demo-full-stack/tasks.toml` T04 | 49.7 C | 74.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0039 |
| 6 | plans | `plans/demos/demo-full-stack/tasks.toml` T05 | 49.7 C | 74.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0050 |
| 7 | plans | `plans/demos/demo-release-readiness/tasks.toml` REL-2 | 54.7 C | 79.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0036 |
| 8 | plans | `plans/demos/demo-fibonacci/tasks.toml` FIB-2 | 54.7 C | 79.7 B | - -> - | 2 | rounds_exhausted | 2 | $0.0040 |
| 9 | plans | `plans/e2e-smoke-test/tasks.toml` T01 | 38.4 D | 73.4 B | - -> - | 2 | rounds_exhausted | 2 | $0.0028 |
| 10 | plans | `plans/demos/demo-hello/tasks.toml` DEMO-T01 | 44.7 C | 90.0 A | - -> - | 2 | rounds_exhausted | 2 | $0.0028 |
| 11 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq10-vagueness/tasks.toml` T3 | 30.0 D | 72.5 B | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0021 |
| 12 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq12-hidden-hook/tasks.toml` T2 | 43.5 C | 77.7 B | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0021 |
| 13 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/hf5-greenfield-claim/tasks.toml` T1 | 46.2 C | 82.5 A | HF5 -> HF4,HF5 | 2 | rounds_exhausted | 2 | $0.0026 |
| 14 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq05-specificity/tasks.toml` T4 | 25.9 D | 62.2 B | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0022 |
| 15 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq09-non-goals/tasks.toml` T2 | 52.5 C | 82.5 A | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0020 |
| 16 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq07-context/tasks.toml` T1 | 51.2 C | 82.5 A | HF4 -> HF4 | 2 | rounds_exhausted | 2 | $0.0016 |
| 17 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq06-red-on-base/tasks.toml` T2 | 46.2 C | 82.5 A | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0019 |
| 18 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq01-goal/tasks.toml` T4 | 58.8 C | 95.0 A | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0017 |
| 19 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/dynamic/red-on-base/tasks.toml` T3 | 23.4 D | 70.0 B | - -> HF2,HF4 | 2 | rounds_exhausted | 2 | $0.0022 |
| 20 | fixtures | `benchmarks/viabilitybench/speclint/fixtures/sq11-anchors/tasks.toml` T3 | 46.2 C | 82.5 A | - -> HF4 | 2 | rounds_exhausted | 2 | $0.0018 |

Files: `manifest.json` (the config and the sample, with each task's spec hash), `refine.jsonl` (a `vb.refine_record/1` per task: scores before and after, each kept round's `spec.refined` record, what the model proposed, the calls, usage and cost) and `ledger.jsonl` (one row per task that called the model).
