# SWE-bench Proxy + C-Factor Demo

Reusable flow for exercising the native benchmark path without Docker, HuggingFace, or a live LLM.

This demo uses `roko bench swe` against the built-in two-task SWE-bench-style smoke dataset. It proves the process wiring:

1. Generate or receive a patch for each benchmark instance.
2. Validate patch format.
3. Run `git apply --check`.
4. Apply the patch.
5. Reset the grading tests over the patch and run the task test command.
6. Persist score rows under `.roko/bench`: `controls.jsonl` for the `gold` and `empty` controls, `scores.jsonl` for agent runs.
7. For agent runs only, persist learning episodes (`.roko/episodes.jsonl`), efficiency events and C-factor snapshots (`.roko/learn`). Controls never write learning state.

This is fast proxy scoring, not official SWE-bench Docker scoring and not comparable to swebench.com.

## Quick Start

From the repo root:

```bash
cargo build -p roko-cli
bash demo/demo-resources/benchmark-flow/demo-benchmark.sh
```

The script needs `git`, `python3`, and a `roko` built from a tree whose bench has the current contract:

- `--agent-mode` is required;
- controls go to `controls.jsonl` and are never learned;
- the agent payload carries no gold patch or tests;
- an agent's unmeasured cost is recorded as `cost_known: false`.

Set `ROKO=/path/to/roko` to use another build. An older build fails the controls check (see Troubleshooting).

To reuse a specific workspace:

```bash
bash demo/demo-resources/benchmark-flow/demo-benchmark.sh /tmp/roko-bench-demo
```

You can also run it through the demo wrapper:

```bash
bash demo/demo-resources/bin/roko-demo run bench
```

## What The Script Runs

`demo-benchmark.sh` runs three batches in the same workspace:

| Batch | Agent mode | Purpose | Expected proxy score |
|---|---|---:|---:|
| Gold control | `--agent-mode gold` | Positive control: applies the dataset's patch. Checks the harness, not a model | `2/2` |
| Empty control | `--agent-mode empty` | Negative control: proves failed patches do not pass | `0/2` |
| Command adapter | `--agent-mode command` | Process adapter smoke: the command receives the instance on stdin and prints a patch | `2/2` |

After the two controls, the script checks that they wrote no episodes, efficiency events or C-factor snapshots.

The command adapter in this demo runs an oracle, not an agent: [`oracle-agent.py`](oracle-agent.py). The agent payload carries only the instance id, repo and problem statement, never the gold patch or the grading tests, so the oracle reads the answer from where the bench keeps it for controls. The gold control's `--export-predictions` file holds it, and the oracle looks up the patch by the `instance_id` it gets on stdin:

```bash
python3 demo/demo-resources/benchmark-flow/oracle-agent.py "$WORKDIR/predictions-gold.jsonl"
```

The oracle exits with an error if the payload ever carries `patch`, `test_cmd`, `test_patch` or `test_files`. The run proves the command-mode plumbing and the learning writes. It is not an agent result. It is recorded like any command-mode run, which is why the demo uses a throwaway workspace. Replace it with your own agent command when testing a real patch-producing process.

## Artifacts

Given `WORKDIR=/tmp/roko-bench-demo`, the demo writes:

```text
$WORKDIR/
├── .roko/
│   ├── episodes.jsonl            # command-run episodes (cost_known: false)
│   ├── bench/
│   │   ├── controls.jsonl        # gold and empty rows, control: true
│   │   ├── scores.jsonl          # command-run row, control: false
│   │   └── runs/*.json           # full per-instance details
│   └── learn/
│       ├── efficiency.jsonl      # command-run efficiency events
│       └── c-factor.jsonl        # C-factor snapshots
├── predictions-gold.jsonl        # the gold control's patches (the oracle's source)
└── predictions-command.jsonl     # what the oracle replayed
```

Inspect the rows:

```bash
tail -n 2 /tmp/roko-bench-demo/.roko/bench/controls.jsonl
tail -n 1 /tmp/roko-bench-demo/.roko/bench/scores.jsonl
```

Inspect the C-factor and contributors:

```bash
./target/debug/roko status --workdir /tmp/roko-bench-demo --cfactor
```

Expected pattern:

- gold resolves 2/2 and empty 0/2. Both rows have `control: true` and no C-factor, and neither writes an episode;
- the command run resolves 2/2, writes two episodes with `extra.cost_known: false`, and creates the first C-factor snapshot;
- `swe-bench-command` is the only C-factor contributor. Its 2/2 is the oracle's, not a model's.

## Custom Dataset JSONL

Pass a local dataset file directly to `roko bench swe`:

```bash
./target/debug/roko bench swe \
  --dataset ./my-swe-smoke.jsonl \
  --batch-size 5 \
  --agent-mode command \
  --agent-command './my-agent-command' \
  --workdir /tmp/roko-bench-custom
```

Each JSONL row should have this shape:

```json
{
  "instance_id": "local__case-1",
  "repo": "local/example",
  "repo_path": "./fixtures/case-1",
  "problem_statement": "Fix the failing behavior.",
  "patch": "diff --git a/file.py b/file.py\n...",
  "test_cmd": "python3 -m unittest",
  "test_files": ["test_file.py"]
}
```

`repo_path` is copied into an isolated benchmark workdir before patch validation. Relative `repo_path` values are resolved relative to the dataset file.

Every row must name the tests that grade it: `test_files` (paths in the repo), a SWE-bench-style `test_patch`, or both. Before `test_cmd` runs, those files are reset over the agent's patch and `test_patch` is applied, so a patch cannot rewrite the tests that grade it. A row with no grading tests, or with no `test_cmd`, is an error. The agent never sees `patch`, `test_cmd`, `test_patch` or `test_files`.

## Agent Modes

| Mode | Flag | Use |
|---|---|---|
| Gold | `--agent-mode gold` | Harness positive control |
| Empty | `--agent-mode empty` | Harness negative control |
| Prediction file | `--agent-mode prediction-file --predictions predictions.jsonl` | Replay existing SWE-bench-style predictions |
| Command | `--agent-mode command --agent-command '<cmd>'` | Wrap a real agent or script |

`--agent-mode` has no default. Control rows go to `controls.jsonl` and are never recorded as learning.

For command mode, Roko writes one instance JSON object to stdin and expects a unified diff on stdout. The object carries `instance_id`, `repo`, `repo_path`, `base_commit` and `problem_statement`, never the gold patch or the grading tests.

## Official SWE-bench

Use this demo for fast local harness verification and learning telemetry. For publishable SWE-bench numbers, export predictions and run the official Python/Docker harness separately:

```bash
./target/debug/roko bench swe \
  --batch-size 300 \
  --agent-mode command \
  --agent-command './my-agent-command' \
  --export-predictions /tmp/predictions.jsonl

python -m swebench.harness.run_evaluation \
  --predictions_path /tmp/predictions.jsonl \
  --dataset_name princeton-nlp/SWE-bench_Lite \
  --run_id roko_proxy_export
```

## Troubleshooting

| Issue | Fix |
|---|---|
| `roko binary not found` | Run `cargo build -p roko-cli` or set `ROKO=/path/to/roko` |
| `git apply` fails for gold mode | Rebuild Roko and rerun; gold mode should always pass the built-in smoke dataset |
| `a control wrote learning state` | Your `roko` predates labeled controls and records gold and empty runs as learning. Rebuild with `cargo build -p roko-cli` |
| `the following required arguments were not provided: --agent-mode` | Pass `--agent-mode`. It has no default, so a forgotten flag cannot quietly run the gold control |
| `KeyError: 'patch'` from an older oracle command | The payload no longer carries the gold patch. Read it from the gold control's `--export-predictions` file, as `oracle-agent.py` does |
| Command mode hangs | Your command likely did not close stdout or is waiting for input; it receives exactly one JSON object on stdin |
| Official SWE-bench score differs from proxy | Expected; proxy validates format/apply/tests locally, while official scoring uses task containers and SWE-bench test metadata |
