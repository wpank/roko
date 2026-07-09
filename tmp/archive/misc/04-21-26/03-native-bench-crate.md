# Native Benchmark Crate: `roko bench`

## Design Principle

SWE-bench instances are just tasks. The orchestrator already does
compose → dispatch → gate → learn → persist. The benchmark harness
is just a **task generator** that feeds instances into the existing pipeline.

No new execution machinery. No Python. The learning loops fire automatically
because the code path goes through `plan run`.

## Architecture

```
roko bench swe --batch-size 50 --repeat 0

  1. Dataset loader (HF Parquet/REST → Vec<SweInstance>)
  2. Instance → Task mapper (each instance = 1 TaskDef)
  3. Plan generator (batch of tasks = 1 plan)
  4. Delegates to plan run (existing orchestrator)

  plan run does:
    compose prompt (SystemPromptBuilder)
    → dispatch agent (CascadeRouter picks model)
    → run gates:
        ShellGate("git apply --check patch")
        ShellGate("cd repo && pytest tests/...")
    → record_completed_run()
        → CascadeRouter.observe()
        → playbook store
        → episode logger
        → efficiency events
        → experiment variants
    → on 3 gate failures → replan

  5. Score aggregator (reads episodes, computes %)
  6. Outer loop (--repeat N, learning carries over)
```

## What Gets Built

### 1. Dataset Loading (~400 lines)

Two strategies:
- **REST**: Dataset Viewer API at `datasets-server.huggingface.co/rows` — stream 100 rows at a time, no local storage needed
- **Parquet**: Download Parquet file via Dataset Viewer `/parquet` endpoint, read with `arrow`/`parquet` crates for bulk access

```rust
struct SweInstance {
    instance_id: String,
    repo: String,           // e.g. "pallets/flask"
    base_commit: String,
    patch: String,          // gold patch (for oracle file extraction)
    problem_statement: String,
    test_cmd: String,       // FAIL_TO_PASS test command
    // ...
}
```

### 2. Repo Preparation (~200 lines)

Use `git2` crate (already a dep via roko-index) for clone + checkout.
Cache clones in `.roko/bench/repos/` to avoid re-cloning across batches.

### 3. Instance-to-Task Mapping (~150 lines)

```rust
fn swe_instance_to_task(inst: &SweInstance, workdir: &Path) -> TaskDef {
    TaskDef {
        id: inst.instance_id.clone(),
        prompt: format!(
            "# Issue\n\n{}\n\nProduce a unified diff that resolves this issue.",
            inst.problem_statement
        ),
        workdir: workdir.to_path_buf(),
        gates: vec![
            GateConfig::Shell { cmd: "git apply --check patch.diff".into() },
            GateConfig::Shell { cmd: inst.test_cmd.clone() },
        ],
        oracle_files: oracle_files(&inst.patch),
        // ...
    }
}
```

### 4. Score Aggregation (~100 lines)

Read `.roko/episodes.jsonl`, filter by plan_id (the batch), count pass/fail.
Append aggregate score to `.roko/bench/scores.jsonl`.

### 5. CLI Subcommand (~200 lines)

```
roko bench swe [OPTIONS]

OPTIONS:
  --dataset <NAME>       HF dataset (default: princeton-nlp/SWE-bench_Lite)
  --split <SPLIT>        Dataset split (default: test)
  --batch-size <N>       Instances per batch (default: 50)
  --repeat <N>           Batches to run (0 = infinite)
  --shuffle              Random sample each batch
  --offset <N>           Start index
  --instance-ids <IDS>   Run specific instances only
  --experiment <ID>      Prompt experiment to run
  --report <PATH>        Score output path (default: .roko/bench/scores.jsonl)
  --export-predictions   Write predictions.jsonl for official harness

EXAMPLES:
  # Quick smoke test
  roko bench swe --batch-size 5

  # Perpetual grinder (runs forever, learns continuously)
  roko bench swe --repeat 0 --batch-size 50 --shuffle

  # A/B prompt experiment
  roko bench swe --batch-size 100 --experiment prompt-style-v2

  # Export for official scoring
  roko bench swe --batch-size 300 --export-predictions
```

## The Perpetual Grinder

```bash
roko bench swe --repeat 0 --batch-size 50 --shuffle
```

Each batch:
1. Samples 50 instances (different each time with --shuffle)
2. CascadeRouter picks models (starts static → confidence → UCB after 200 obs)
3. Prompt experiments vary sections (UCB1 converges on winners)
4. Gates run per-instance tests
5. Failures trigger replan (retry with different approach after 3 failures)
6. Playbooks from successful patches get injected into future prompts
7. Scores logged, next batch starts with updated weights

No external scripts. No Python. The learning loops fire because the code path goes
through `plan run`, which already wires all of them.

## What About Official Scoring?

The official SWE-bench harness (`python -m swebench.harness.run_evaluation`) needs Docker + Python.

Two-tier approach:
- **Fast proxy (every batch)**: ShellGate with `git apply --check` + instance test command.
  This is what the grinder uses — fast iteration matters more than exact scoring.
- **Official scoring (periodic)**: `roko bench swe --export-predictions` writes `predictions.jsonl`.
  Run the Python harness nightly/weekly as validation. Do this for publishable numbers, not for learning.

## Estimated Line Counts

| Component | Lines |
|-----------|-------|
| Dataset loading (REST + Parquet) | ~400 |
| Repo preparation (git2 clone/checkout) | ~200 |
| Instance-to-task mapping | ~150 |
| Score aggregation | ~100 |
| CLI subcommand + config | ~200 |
| Tests | ~300 |
| **Total** | **~1350** |

This is a single crate (`roko-bench`) or a module within `roko-cli`.
