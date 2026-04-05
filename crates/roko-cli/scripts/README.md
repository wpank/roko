# roko-cli scripts

Helper scripts that sit on top of the `roko` binary.

## `swebench_run.py` — SWE-bench-Lite driver

A single-shot driver that:

1. Loads the [SWE-bench-Lite](https://www.swebench.com/) dataset (300 real
   GitHub issues) from HuggingFace
2. For each task, does **oracle retrieval** — uses the files touched by
   the gold patch as the files injected into the prompt
3. Clones each task's repo at the correct base commit
4. Writes a per-task `roko.toml` with issue text + oracle files + your
   chosen model
5. Runs `roko run` and extracts the unified diff from the agent output
6. Writes a `predictions.jsonl` in the format the official SWE-bench
   evaluation harness expects

It does **not** score the predictions itself — that's the harness's job
(see step 3 below). It only produces the input.

### Prerequisites

```bash
# 1. Install the roko binary
cd roko && cargo install --path crates/roko-cli

# 2. Install Python deps
pip install datasets
pip install swebench         # only needed for scoring, step 3

# 3. Ollama running with at least one model pulled
ollama pull llama3.2:latest
ollama serve                 # or the app/launchd on macOS
```

Docker is required for the official SWE-bench harness (it runs each repo's
tests in per-task containers). If you just want the predictions JSONL,
Docker is optional.

### Step 1 — generate predictions (fastest smoke test)

```bash
cd crates/roko-cli/scripts

python3 swebench_run.py \
  --model llama3.2:latest \
  --limit 5 \
  --output /tmp/preds-llama3.2.jsonl
```

This runs 5 SWE-bench-Lite tasks through llama3.2. Expect ~2-10 minutes
depending on your hardware. The driver prints a summary per task:

```
=== pallets__flask-4045 ===
    oracle files: ['src/flask/blueprints.py']
    patch extracted: 267 bytes
```

### Step 2 — inspect what the model produced

```bash
# Latest agent output for any instance:
jq -r 'select(.kind=="agent_output" and .tags.cleaned=="true") | .body.data' \
  /tmp/roko-swe-workdirs/pallets__flask-4045/.roko/signals.jsonl \
  | tail -n 1 | jq -r .

# Full predictions file:
cat /tmp/preds-llama3.2.jsonl | jq .
```

### Step 3 — score with the official SWE-bench harness

```bash
python -m swebench.harness.run_evaluation \
  --predictions_path /tmp/preds-llama3.2.jsonl \
  --dataset_name princeton-nlp/SWE-bench_Lite \
  --run_id llama3.2_smoke \
  --max_workers 4
```

The harness spins up one Docker container per task, applies the patch,
runs the `FAIL_TO_PASS` + `PASS_TO_PASS` test lists, and reports:

- **resolved**: patch applied AND all designated tests now pass
- **unresolved**: patch applied but tests failed
- **error**: patch failed to apply or container errored

Your score is `resolved / total` (typically reported as a percentage).
Post this number with your run config (model, tokens, prompt) to compare
against entries on [swebench.com](https://www.swebench.com).

### Flags

| Flag | Default | Notes |
|---|---|---|
| `--model` | *required* | Ollama model tag, e.g. `llama3.2:latest` |
| `--backend` | `ollama` | Backend CLI (any stdin/stdout LLM CLI) |
| `--dataset` | `princeton-nlp/SWE-bench_Lite` | HF dataset name |
| `--split` | `test` | Dataset split |
| `--limit` | `5` | Number of instances to run |
| `--offset` | `0` | Start index into the dataset |
| `--instance-ids` | — | Run specific ids (overrides limit/offset) |
| `--output` | `predictions.jsonl` | Output path (append mode) |
| `--workdir-root` | `/tmp/roko-swe-workdirs` | Per-instance workdirs |
| `--token-budget` | `20000` | Composer budget per prompt |
| `--timeout-ms` | `600000` | Per-task model timeout (10 min) |
| `--file-hard-cap` | `4000` | Per-file token cap (truncates large files) |
| `--roko-bin` | `roko` | Path to the roko binary |

### Examples

```bash
# Run 10 tasks with llama3.2
python3 swebench_run.py --model llama3.2:latest --limit 10

# Run ALL 300 SWE-bench-Lite tasks (slow — many hours)
python3 swebench_run.py --model llama3.2:latest --limit 300

# Re-run only flask tasks:
python3 swebench_run.py --model gemma4:26b-moe-nothink \
  --instance-ids pallets__flask-4045 pallets__flask-4992 pallets__flask-5063

# Use SWE-bench full (2294 tasks, multi-day run)
python3 swebench_run.py --model llama3.2:latest \
  --dataset princeton-nlp/SWE-bench --limit 20

# Skip ahead past the first 50 tasks
python3 swebench_run.py --model llama3.2:latest --offset 50 --limit 10

# Cap per-file budget very tightly (for small-context models)
python3 swebench_run.py --model llama3.2:latest --limit 5 \
  --file-hard-cap 1500 --token-budget 8000
```

### What this driver does NOT do

These are things "real" SWE-bench agents do that we intentionally skip to
keep the driver minimal:

- **Tool use / multi-turn exploration.** Agents like SWE-agent, Aider,
  Cognition's Devin, etc. let the model run `grep`, `find`, `read_file`,
  `edit_file` in loops. This driver gives the model one shot with the
  oracle files only.
- **Retrieval beyond oracle.** Real benchmarks fairness-wise also evaluate
  with BM25 or embedding retrieval (no knowledge of the gold patch).
  Oracle retrieval **upper-bounds** what you'd get with smarter retrieval,
  which is why it's a fair calibration baseline.
- **Reflection / self-repair.** No second pass if the patch fails to
  apply or tests fail.
- **Patch repair heuristics.** Some harnesses post-process model output
  to fix common mistakes (e.g. missing `a/`/`b/` prefixes). We don't.

### Expected score range for local ollama models

**Be realistic.** SWE-bench-Lite is hard. The current leaderboard shows:

- Frontier models (Claude 4.5 Sonnet, GPT-5, with tool use): 55-80%
- Frontier models (zero-shot, oracle retrieval): 20-35%
- Open 70B models (zero-shot, oracle retrieval): 3-10%
- Open 7-30B models (zero-shot, oracle retrieval): **0-3%**

Your local ollama models sit in the bottom bucket. Typical failure modes:

- **Malformed diff output** — wrong hunk headers, missing `a/`/`b/` path
  prefixes, extra prose around the diff. Even when the logical fix is
  correct, the harness rejects syntactically invalid patches. This
  dominates small-model failures.
- **Editing wrong file or wrong line** — models pick a plausible-looking
  location that isn't what the gold patch touches.
- **Timeouts** — 28GB models on CPU can take 5-10+ minutes per task.
  Budget accordingly.

If you get even 1-2% resolved on SWE-bench-Lite with a single-shot
local-model pipeline, that's roughly state-of-the-art for that class of
setup. The numbers go up substantially with (a) tool use, (b) much
bigger models, (c) multi-pass / reflection loops.

### Testing the script itself

```bash
python3 test_swebench_run.py
```

Runs 16 unit tests covering oracle-file parsing, patch extraction (fenced
+ unfenced diffs), signal-log parsing, and config generation (including
all ablation-knob combinations). No network or subprocess dependencies.

---

## Measuring whether the harness actually helps

A harness only earns its keep if it **moves a metric**. To attribute
score changes to roko-cli specifically (vs. the underlying LLM's baseline
performance), compare controlled runs with the same model + same tasks.

Three scripts compose to answer "does my harness help?":

| Script | Role |
|---|---|
| `swebench_run.py` | **Harness** — full roko-cli pipeline |
| `swebench_baseline.py` | **Control** — raw `ollama run`, no harness |
| `swebench_validate.py` | **Scorer** — local, Docker-free, fast |

### A/B recipe

Run both pipelines against the same instance list, then compare:

```bash
# Pick 10 instances (deterministic: first 10 of SWE-bench-Lite test split)
LIMIT=10
MODEL=llama3.2:latest

# ── Treatment: full roko-cli harness ──────────────────────────
rm -f /tmp/preds-harness.jsonl
python3 swebench_run.py \
  --model $MODEL --limit $LIMIT \
  --output /tmp/preds-harness.jsonl \
  --workdir-root /tmp/roko-ab/harness \
  --suffix full-harness

# ── Control: raw ollama, same tasks, no harness ───────────────
rm -f /tmp/preds-baseline.jsonl
python3 swebench_baseline.py \
  --model $MODEL --limit $LIMIT \
  --output /tmp/preds-baseline.jsonl \
  --workdir-root /tmp/roko-ab/baseline

# ── Score both locally (no Docker) ────────────────────────────
python3 swebench_validate.py \
  --predictions /tmp/preds-harness.jsonl /tmp/preds-baseline.jsonl
```

Example output:

```
=== /tmp/preds-harness.jsonl (10 predictions) ===
  format_valid    : 3/10   (30.0%)
  apply_check_ok  : 1/10   (10.0%)
  touches_oracle  : 3/10   (30.0%)
  patch bytes     : avg=1342 median=1100 min=0 max=4200

=== /tmp/preds-baseline.jsonl (10 predictions) ===
  format_valid    : 1/10   (10.0%)
  apply_check_ok  : 0/10   (0.0%)
  touches_oracle  : 1/10   (10.0%)
  patch bytes     : avg=1580 median=1400 min=400 max=3100

=== A/B delta ===
metric            preds-harness.jsonl    preds-baseline.jsonl   delta(2-1)
format_valid                    30.0%                    10.0%    -20.0pp
apply_check                     10.0%                     0.0%    -10.0pp
touches_oracle                  30.0%                    10.0%    -20.0pp
```

Negative deltas mean **the harness helped**: baseline minus harness. If
the delta is zero or positive, the harness is not earning its keep for
that (model, task) combination.

### Metrics explained

| Metric | What it measures | Failure modes caught |
|---|---|---|
| `format_valid` | Output has `diff --git` header + `@@` hunk | Prose responses, wrong prefix (`originally`/`modified`), missing hunks |
| `apply_check` | `git apply --check` accepts the patch | Wrong line numbers, missing context lines, malformed hunks |
| `touches_oracle` | Patch touches the same file(s) as gold patch | Model edits the wrong file |
| `patch_bytes` | Distribution of patch sizes | Empty outputs, runaway generation |

`apply_check` is a strict **upper bound** on the official SWE-bench
`resolved` rate: a prediction that can't apply can't resolve. So if your
harness improves `apply_check` from 0% to 10%, the best-case
`resolved` improvement is +10 percentage points. The official harness
will then also filter out patches that apply but fail the task's tests.

### Ablation study: attributing to individual features

`swebench_run.py` has four ablation flags that let you turn off specific
harness features in isolation:

| Flag | What it disables |
|---|---|
| `--no-clean-output` | ANSI + thinking-trace stripping (raw stdout persisted) |
| `--no-file-injection` | `[[prompt.files]]` — files get crammed into prompt text instead |
| `--no-hard-cap` | Per-file token caps |
| `--minimal-role` | 1-line role instead of structured instructions |

Run an ablation study over the same 10 instances:

```bash
LIMIT=10; MODEL=llama3.2:latest

# Full harness
python3 swebench_run.py --model $MODEL --limit $LIMIT \
  --output /tmp/abl-full.jsonl --workdir-root /tmp/roko-ab/full \
  --suffix full

# Drop clean_output
python3 swebench_run.py --model $MODEL --limit $LIMIT --no-clean-output \
  --output /tmp/abl-noclean.jsonl --workdir-root /tmp/roko-ab/noclean \
  --suffix no-clean

# Drop file injection
python3 swebench_run.py --model $MODEL --limit $LIMIT --no-file-injection \
  --output /tmp/abl-nofiles.jsonl --workdir-root /tmp/roko-ab/nofiles \
  --suffix no-files

# Minimal role
python3 swebench_run.py --model $MODEL --limit $LIMIT --minimal-role \
  --output /tmp/abl-minrole.jsonl --workdir-root /tmp/roko-ab/minrole \
  --suffix min-role

# Score all four
python3 swebench_validate.py \
  --predictions /tmp/abl-full.jsonl /tmp/abl-noclean.jsonl \
                /tmp/abl-nofiles.jsonl /tmp/abl-minrole.jsonl
```

Look at each ablation's `format_valid` / `apply_check` rates. The feature
whose removal drops the metric the most is the one contributing most
value for that (model, task, LIMIT) combination.

### When the A/B will show a delta (and when it won't)

**Expect clear deltas when:**
- The model is on the capability boundary for patch formatting. Mid-range
  7B-34B models often produce "close to valid" diffs; the harness's
  clean_output / structured role can push them over the line.
- You're running enough instances (10+) for the noise floor to shrink.
  Rates computed on N=3 are dominated by variance.
- Tasks are small (short issue text, short file). Big files force small
  models into failure modes the harness can't rescue.

**Don't expect deltas when:**
- Model is too weak (like llama3.2:latest at 2GB — it produces
  malformed diffs no matter the prompt).
- Model is too strong (frontier models produce valid diffs even from
  minimal prompts).
- N is small enough that noise dominates.
- Task requires multi-file edits (oracle retrieval + the harness both
  assume the gold patch's files are sufficient context).

### Caveats

- **Local validation is an upper bound, not the final score.** The
  official harness runs each task's tests inside Docker — some patches
  that pass `git apply --check` still fail the tests. Real `resolved`
  rates are always `≤ apply_check` rate.
- **Results are per-`(model, dataset, LIMIT, seed)` combination.** Don't
  generalize from 10 instances to a swebench.com-style headline number.
- **Ollama is not deterministic.** Temperature, top_p defaults can
  produce different outputs on repeated runs. If you're doing statistical
  claims, run each cell 3-5 times and report variance.
- **Prompt variants matter.** `--minimal-role` vs. the default is a huge
  delta; people overreport "my harness helped by X%" when they're
  secretly comparing different prompts.
