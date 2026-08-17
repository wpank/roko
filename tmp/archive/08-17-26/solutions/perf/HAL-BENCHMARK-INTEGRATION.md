# HAL Benchmark Integration Plan

Date: 2026-04-29
Status: Research complete, implementation plan drafted
References:
- HAL Leaderboard: https://hal.cs.princeton.edu/
- HAL Harness: https://github.com/princeton-pli/hal-harness
- Paper: https://arxiv.org/abs/2510.11977 (Accepted at ICLR 2026)

---

## 1. What is HAL?

The Holistic Agent Leaderboard (HAL), developed at Princeton, is a standardized
evaluation harness for reproducible AI agent evaluations. It addresses the core
problem of agent benchmarking: every agent framework has its own evaluation setup,
making cross-framework comparison unreliable.

### Key properties

1. **Unified CLI**: Single `hal-eval` command works across all supported benchmarks
2. **Reproducible**: Docker containers or Azure VMs for isolation
3. **Parallel**: Orchestrates hundreds of concurrent evaluations
4. **Multi-dimensional**: Measures accuracy, consistency, cost, and safety
5. **Framework-agnostic**: Agents are simple callables, no framework dependency

### Benchmarks included

| Benchmark | Domain | Tasks | What it measures |
|---|---|---|---|
| SWE-bench Verified | Coding | 500 | Real GitHub issue resolution |
| SWE-bench mini | Coding | 50 | Subset for quick iteration |
| USACO | Coding | ~300 | Competitive programming |
| SciCode | Science | 338 | Scientific computation |
| ScienceAgentBench | Science | 44 | Data-driven discovery |
| CORE-bench Hard | Science | 270 | Scientific reproducibility |
| GAIA | General | 166 | General assistant tasks |
| AssistantBench | Web | 33 | Web search evaluation |
| tau-bench | Service | 200 | Tool-agent-user interaction |
| AppWorld | Tools | 750 | Complex function calling |
| CollaborativeAgentBench | Collab | -- | Human-agent collaboration |

### Scale

The HAL team ran 21,730 agent rollouts across 9 models and 9 benchmarks for their
initial leaderboard at a total cost of ~$40,000. This validates the harness can
handle large-scale evaluation.

---

## 2. Why Integrate with HAL?

### 2.1 External validation of roko's agent quality

Roko's self-evaluation loop (gate pipeline, efficiency signals, cascade router) is
internal. It measures whether agents produce code that compiles and passes tests,
but it cannot measure how roko agents compare to other agent frameworks on
standardized tasks.

HAL provides this external calibration:
- **SWE-bench**: How well does roko's orchestration resolve real GitHub issues?
- **USACO**: How well does roko handle complex algorithmic tasks?
- **CORE-bench**: Can roko reproduce scientific results?

### 2.2 Performance benchmarking across models

Roko supports 8+ LLM backends (Claude, GPT-4, Gemini, Moonshot, Ollama, Cerebras,
Perplexity, Codex). HAL's multi-model evaluation can systematically compare:
- Which backend produces the best results for which task type
- Cost/quality Pareto frontiers across providers
- Latency vs. accuracy tradeoffs

This data feeds directly into roko's `CascadeRouter` to improve model selection.

### 2.3 Regression detection

Running HAL benchmarks on each release provides a quality regression signal:
- Did the prompt assembly change improve or degrade SWE-bench scores?
- Did the gate pipeline optimization affect task completion rates?
- Does the new model routing strategy pick better models?

---

## 3. Integration Architecture

### 3.1 Roko as a HAL agent

HAL agents are Python callables with the signature:

```python
def run(task: dict, **agent_args) -> dict:
    """Execute a benchmark task and return results."""
    ...
```

Roko's integration wraps the Rust CLI binary in a Python shim:

```
                    ┌───────────────────────┐
                    │    HAL Harness         │
                    │    (Python 3.12)       │
                    └──────────┬────────────┘
                               │
                               ▼
                    ┌───────────────────────┐
                    │    roko_hal_agent.py   │
                    │    (Python wrapper)    │
                    └──────────┬────────────┘
                               │
                    ┌──────────▼────────────┐
                    │    roko CLI            │
                    │    (Rust binary)       │
                    │                        │
                    │  roko run --model X    │
                    │    --gates express     │
                    │    --output json       │
                    │    "<task prompt>"     │
                    └───────────────────────┘
```

### 3.2 Agent wrapper implementation

**File to create**: `hal/roko_agent/main.py`

```python
"""Roko agent wrapper for HAL benchmark harness."""

import json
import os
import subprocess
import tempfile
import time
from pathlib import Path


def run(task: dict, **kwargs) -> dict:
    """Execute a HAL benchmark task using roko.

    Args:
        task: Benchmark task dict with 'instance_id', 'prompt', 'repo', etc.
        **kwargs: Agent arguments from -A flags.
            model_name: LLM model to use (default: gpt-4.1-mini)
            workflow: Workflow template (default: express)
            gates: Gate mode (default: express)
            timeout: Max seconds per task (default: 300)
            roko_binary: Path to roko binary (default: roko)

    Returns:
        Dict with 'model_patch' (diff), 'cost', 'tokens', 'duration_s'.
    """
    model = kwargs.get("model_name", "gpt-4.1-mini")
    workflow = kwargs.get("workflow", "express")
    gates = kwargs.get("gates", "express")
    timeout = int(kwargs.get("timeout", "300"))
    roko_bin = kwargs.get("roko_binary", "roko")

    # Set up workspace
    workdir = setup_workspace(task)

    # Build prompt from task
    prompt = build_prompt(task)

    # Run roko
    start = time.time()
    result = subprocess.run(
        [
            roko_bin, "run",
            "--model", model,
            "--workflow-template", workflow,
            "--gates", gates,
            "--output", "json",
            prompt,
        ],
        cwd=workdir,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    duration = time.time() - start

    # Parse output
    output = parse_roko_output(result)

    # Get git diff as the "model patch"
    diff = get_git_diff(workdir)

    return {
        "model_patch": diff,
        "cost": output.get("cost_usd", 0.0),
        "tokens": output.get("total_tokens", 0),
        "duration_s": duration,
        "model": model,
        "workflow": workflow,
        "exit_code": result.returncode,
    }


def setup_workspace(task: dict) -> str:
    """Clone or prepare the task repository."""
    if "repo" in task:
        workdir = tempfile.mkdtemp(prefix="roko-hal-")
        subprocess.run(
            ["git", "clone", "--depth", "1", task["repo"], workdir],
            check=True, capture_output=True,
        )
        if "base_commit" in task:
            subprocess.run(
                ["git", "checkout", task["base_commit"]],
                cwd=workdir, check=True, capture_output=True,
            )
        # Initialize roko in the workspace
        subprocess.run(
            ["roko", "init"], cwd=workdir, capture_output=True,
        )
        return workdir
    return task.get("workdir", os.getcwd())


def build_prompt(task: dict) -> str:
    """Build a prompt from the task description."""
    parts = []
    if "problem_statement" in task:
        parts.append(task["problem_statement"])
    elif "prompt" in task:
        parts.append(task["prompt"])
    elif "issue" in task:
        parts.append(f"Fix this issue:\n\n{task['issue']}")

    if "hints" in task:
        parts.append(f"\nHints:\n{task['hints']}")

    return "\n\n".join(parts)


def parse_roko_output(result: subprocess.CompletedProcess) -> dict:
    """Parse roko's JSON output."""
    try:
        return json.loads(result.stdout)
    except (json.JSONDecodeError, ValueError):
        return {"raw_stdout": result.stdout, "raw_stderr": result.stderr}


def get_git_diff(workdir: str) -> str:
    """Get the git diff produced by roko."""
    result = subprocess.run(
        ["git", "diff", "HEAD"],
        cwd=workdir, capture_output=True, text=True,
    )
    return result.stdout if result.returncode == 0 else ""
```

### 3.3 HAL invocation

```bash
# Install HAL harness
pip install hal-harness

# Run roko on SWE-bench mini (50 tasks)
hal-eval \
  --benchmark swe_bench_verified_mini \
  --agent_dir hal/roko_agent/ \
  --agent_function main.run \
  --agent_name "roko (gpt-4.1-mini)" \
  -A model_name=gpt-4.1-mini \
  -A workflow=standard \
  -A gates=express \
  -A roko_binary=./target/release/roko \
  --max_concurrent 5

# Run roko on USACO
hal-eval \
  --benchmark usaco \
  --agent_dir hal/roko_agent/ \
  --agent_function main.run \
  --agent_name "roko (claude-sonnet-4)" \
  -A model_name=claude-sonnet-4 \
  -A workflow=full \
  --max_concurrent 3
```

---

## 4. Roko-Native Benchmarking

Beyond HAL, roko has its own built-in bench system at `crates/roko-serve/src/bench.rs`.
This section describes how to extend it for comprehensive performance and quality
evaluation.

### 4.1 Existing bench infrastructure

**File**: `crates/roko-serve/src/bench.rs`

The `BenchSuite` type defines a benchmark suite:
```rust
pub struct BenchSuite {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tasks: Vec<BenchTask>,
}

pub struct BenchTask {
    pub id: String,
    pub prompt: String,
    pub expected: Option<String>,
    pub tags: Vec<String>,
}
```

Results are stored in `.roko/bench/` as JSON with timing, pass/fail, and token usage.

### 4.2 Performance benchmark suite

Create a performance-focused suite that measures roko overhead, not model quality:

**File to create**: `.roko/bench/suites/perf.json`

```json
{
  "id": "perf-overhead",
  "name": "Performance Overhead Measurement",
  "description": "Measures roko's non-inference overhead across workflows",
  "tasks": [
    {
      "id": "perf-001",
      "prompt": "Reply with only the word hello",
      "tags": ["minimal", "no-tools", "baseline"],
      "expected": "hello",
      "config": {
        "workflow": "express",
        "gates": "none",
        "model": "gpt-4.1-nano"
      }
    },
    {
      "id": "perf-002",
      "prompt": "Create a file called test.txt with the content 'hello world'",
      "tags": ["single-tool", "file-write"],
      "config": {
        "workflow": "express",
        "gates": "none",
        "model": "gpt-4.1-nano"
      }
    },
    {
      "id": "perf-003",
      "prompt": "Add a comment to main.rs: // Performance test",
      "tags": ["code-edit", "with-gates"],
      "config": {
        "workflow": "express",
        "gates": "express",
        "model": "gpt-4.1-mini"
      }
    },
    {
      "id": "perf-004",
      "prompt": "Implement a function that returns the sum of two numbers",
      "tags": ["code-gen", "full-gates"],
      "config": {
        "workflow": "standard",
        "gates": "full",
        "model": "gpt-4.1-mini"
      }
    },
    {
      "id": "perf-005",
      "prompt": "Run 3 sequential tasks: create file, edit file, verify file exists",
      "tags": ["multi-step", "sequential"],
      "config": {
        "workflow": "full",
        "gates": "express",
        "model": "gpt-4.1-mini"
      }
    }
  ]
}
```

### 4.3 Quality benchmark suite (HAL-inspired)

Create a quality suite modeled on HAL's multi-dimensional evaluation:

**File to create**: `.roko/bench/suites/quality.json`

```json
{
  "id": "quality-regression",
  "name": "Quality Regression Detection",
  "description": "Tracks agent quality across releases",
  "tasks": [
    {
      "id": "qual-001",
      "prompt": "Fix the compilation error in this Rust code:\n\nfn main() {\n    let x: i32 = \"hello\";\n    println!(\"{}\", x);\n}\n",
      "tags": ["compile-fix", "rust", "easy"],
      "expected_gates": ["compile:pass"]
    },
    {
      "id": "qual-002",
      "prompt": "Write a function that reverses a string in Rust, handling Unicode correctly",
      "tags": ["code-gen", "rust", "medium"],
      "expected_gates": ["compile:pass", "test:pass"]
    },
    {
      "id": "qual-003",
      "prompt": "Refactor this function to use iterators instead of manual loops:\n\nfn sum_even(nums: &[i32]) -> i32 {\n    let mut total = 0;\n    for i in 0..nums.len() {\n        if nums[i] % 2 == 0 {\n            total += nums[i];\n        }\n    }\n    total\n}",
      "tags": ["refactor", "rust", "medium"],
      "expected_gates": ["compile:pass", "clippy:pass"]
    }
  ]
}
```

### 4.4 Metrics to track

For each benchmark run, capture:

| Metric | Source | Use |
|---|---|---|
| Wall clock time | `/usr/bin/time` | Total overhead |
| Inference time | `FeedbackEvent::ModelCall.latency_ms` | Network/model cost |
| Overhead time | wall_clock - inference_time | Roko framework cost |
| Token usage | `ModelCallResponse.usage` | Cost estimation |
| Cost (USD) | `usage.cost_usd` | Budget tracking |
| Gate pass rate | `GateReport.verdicts` | Quality signal |
| Retry count | `PipelineStateV2.iteration` | Efficiency signal |
| Files changed | `EffectDriver.count_changed_files()` | Scope signal |

### 4.5 Pareto analysis

Compare models on the cost-quality frontier:

```
Quality (SWE-bench pass rate)
    ^
    |     *claude-opus-4
    |   *claude-sonnet-4
    |  *gpt-4.1
    | *gpt-4.1-mini
    |*gemini-flash     *kimi-k2-6
    |
    +──────────────────────> Cost ($/task)
```

The cascade router should route tasks to models on the Pareto frontier,
preferring cheaper models when quality is comparable.

---

## 5. Consistency and Reliability Metrics

HAL evaluates agents on multiple dimensions beyond raw accuracy. Roko should
track these for its own agents:

### 5.1 Consistency (run-to-run variance)

Run the same task 10 times with the same model. Measure:
- Pass rate (should be >90% for easy tasks)
- Output similarity (diff between runs)
- Token usage variance (should be <30% CV)

```bash
for i in $(seq 1 10); do
  roko run --model gpt-4.1-mini --seed $i "reverse a string" \
    2>.roko/bench/consistency/run_$i.json
done
```

### 5.2 Robustness (prompt variation)

Test the same task with semantically equivalent but syntactically different prompts:
- "Fix the compilation error"
- "Make this code compile"
- "The code has a type mismatch, fix it"
- "Resolve the compiler error E0308"

All should produce equivalent results.

### 5.3 Safety (tool use boundaries)

Test that agents respect safety contracts:
- Attempt to read files outside the workspace -> should be blocked
- Attempt to run dangerous commands -> should be blocked
- Attempt to modify git history -> should be blocked

### 5.4 Self-awareness (error detection)

Test that agents correctly identify when they cannot solve a task:
- Impossible tasks -> should report failure, not produce garbage
- Ambiguous tasks -> should ask for clarification or make reasonable assumptions
- Out-of-scope tasks -> should decline gracefully

---

## 6. CI Integration

### 6.1 Nightly HAL runs

```yaml
# .github/workflows/hal-bench.yml
name: HAL Benchmark
on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC daily
  workflow_dispatch:

jobs:
  hal-bench:
    runs-on: ubuntu-latest
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v4
      - name: Build roko
        run: cargo build --release -p roko-cli
      - name: Setup HAL
        run: |
          pip install hal-harness
          cp -r hal/roko_agent/ /tmp/roko_agent/
      - name: Run SWE-bench mini
        run: |
          hal-eval \
            --benchmark swe_bench_verified_mini \
            --agent_dir /tmp/roko_agent/ \
            --agent_function main.run \
            --agent_name "roko-nightly" \
            -A model_name=gpt-4.1-mini \
            -A roko_binary=$PWD/target/release/roko \
            --max_concurrent 5
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: hal-results-${{ github.sha }}
          path: hal_results/
```

### 6.2 Per-PR perf regression check

```yaml
# .github/workflows/perf-check.yml
name: Performance Regression Check
on: pull_request

jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build --release -p roko-cli
      - name: Run perf suite
        run: |
          ./target/release/roko bench run .roko/bench/suites/perf.json \
            --output .roko/bench/results/pr-${{ github.event.number }}.json
      - name: Compare with main
        run: |
          ./target/release/roko bench compare \
            .roko/bench/results/main-latest.json \
            .roko/bench/results/pr-${{ github.event.number }}.json \
            --threshold 20  # fail if >20% regression
```

---

## 7. Files to Create/Modify

| File | Purpose | Effort |
|------|---------|--------|
| `hal/roko_agent/main.py` | **NEW** -- HAL agent wrapper | 4h |
| `hal/roko_agent/requirements.txt` | **NEW** -- Python dependencies | 5min |
| `hal/README.md` | **NEW** -- HAL integration docs | 1h |
| `.roko/bench/suites/perf.json` | **NEW** -- Performance benchmark suite | 2h |
| `.roko/bench/suites/quality.json` | **NEW** -- Quality benchmark suite | 2h |
| `crates/roko-serve/src/bench.rs` | Extend with comparison and Pareto analysis | 4h |
| `crates/roko-cli/src/commands/mod.rs` | Add `bench compare` subcommand | 2h |
| `.github/workflows/hal-bench.yml` | **NEW** -- Nightly HAL CI | 1h |
| `.github/workflows/perf-check.yml` | **NEW** -- PR perf regression check | 1h |

**Total estimated effort**: 17-20h

---

## 8. Expected Outcomes

### Short-term (Week 1-2)

- HAL wrapper working for SWE-bench mini
- First pass rate numbers for roko vs. baseline agents
- Performance benchmark suite running locally

### Medium-term (Month 1-2)

- Nightly HAL runs in CI
- Cascade router tuned using HAL quality data
- Performance regression detection in PRs

### Long-term (Quarter 1-2)

- Full HAL leaderboard submission
- Multi-model Pareto analysis driving automatic model selection
- Consistency and safety metrics tracked per release
- Community benchmark contributions via `roko bench submit`
