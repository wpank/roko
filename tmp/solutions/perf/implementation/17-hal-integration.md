# 17 — HAL (Holistic Agent Leaderboard) Integration

> External quality benchmark integration. Allows roko to participate in
> Princeton's HAL leaderboard and gain external calibration of agent
> quality.
>
> Effort: 17-20 h. Risk: low (purely additive — new files only).

---

## Goal & success criteria

After this change:

1. A Python wrapper at `hal/roko_agent/main.py` exposes roko as a HAL
   agent (`run(task, **kwargs) -> dict`).
2. `roko run --output json` produces machine-parseable output the
   wrapper can consume.
3. `hal-eval --benchmark swe_bench_verified_mini --agent_dir
   hal/roko_agent/ ...` completes against a 50-task subset.
4. A nightly GitHub Actions workflow runs HAL against `main` and
   uploads the results.

Done when:

- `hal-eval ... --agent_name "roko-test"` against a single mock task
  produces a `model_patch` dict with the expected keys.
- A 50-task SWE-bench mini run produces a pass-rate number that we
  can post to the leaderboard.
- The CI workflow exists and runs (manually triggered for the first
  time).

---

## Background

- Source: `HAL-BENCHMARK-INTEGRATION.md` (full plan in this folder),
  `HAL-AND-AGENT-BENCHMARKS.md` §1 (research context).
- HAL is a Python harness; roko is Rust. The integration uses a thin
  subprocess wrapper.
- HAL accepts agents conforming to a tiny protocol:

  ```python
  def run(task: dict, **agent_args) -> dict
  ```

  …with the return dict carrying `model_patch` (the diff), `cost`,
  `tokens`, `duration_s`.
- HAL benchmarks include SWE-bench Verified, USACO, CORE-Bench Hard,
  GAIA, AppWorld, etc. Start with `swe_bench_verified_mini` (50 tasks)
  for cost/time reasons.

---

## Files to read first

| File | Why |
|---|---|
| `HAL-BENCHMARK-INTEGRATION.md` (in this folder) | Full architectural context. |
| HAL harness GitHub: https://github.com/princeton-pli/hal-harness | Agent protocol, benchmark setup, CLI flags. |
| `crates/roko-cli/src/main.rs` | CLI entry — confirm `--output json` exists or add it. |
| `crates/roko-cli/src/output_format.rs` | Output formatter; where JSON output lives. |

---

## Code-level plan

### Step 1 — Ensure `--output json` produces structured output

Verify (or add) that `roko run --output json "..."` emits a JSON object
on stdout containing at least:

```json
{
  "success": true,
  "exit_code": 0,
  "duration_ms": 1234,
  "model": "gpt-4.1-mini",
  "cost_usd": 0.012,
  "input_tokens": 1024,
  "output_tokens": 256,
  "files_changed": ["src/foo.rs"]
}
```

If the formatter writes other lines to stdout (logging, progress),
push them to stderr in `--output json` mode. The HAL wrapper assumes
the entire stdout is parseable JSON.

### Step 2 — Create the HAL wrapper

Folder: `hal/roko_agent/`.

**`hal/roko_agent/main.py`** — copy from
`HAL-BENCHMARK-INTEGRATION.md` §3.2 with these adjustments:

```python
import json
import os
import shlex
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any


def run(task: dict[str, Any], **kwargs: Any) -> dict[str, Any]:
    """Execute a HAL benchmark task using roko.

    Args:
        task: HAL task dict with keys like `instance_id`, `repo`,
              `base_commit`, `problem_statement` (SWE-bench format).
        **kwargs: HAL `-A` agent arguments.
            model_name: model slug to dispatch (default: gpt-4.1-mini).
            workflow:   express|standard|full (default: standard).
            gates:      full|express|none|auto (default: express).
            timeout:    seconds (default: 600).
            roko_binary: path to the roko binary (default: "roko").

    Returns:
        Dict in the HAL response format:
            model_patch  -- unified diff string applied to the workspace
            cost         -- USD cost from `usage.cost_usd`
            tokens       -- input + output tokens
            duration_s   -- wall-clock time
            model        -- model slug actually used
            workflow     -- workflow template used
            exit_code    -- roko CLI exit code
    """
    model = kwargs.get("model_name", "gpt-4.1-mini")
    workflow = kwargs.get("workflow", "standard")
    gates = kwargs.get("gates", "express")
    timeout = int(kwargs.get("timeout", "600"))
    roko_bin = kwargs.get("roko_binary", "roko")

    workdir = _setup_workspace(task)
    prompt = _build_prompt(task)

    start = time.time()
    cmd = [
        roko_bin, "run",
        "--model", model,
        "--workflow-template", workflow,
        "--gates", gates,
        "--output", "json",
        prompt,
    ]
    proc = subprocess.run(
        cmd, cwd=workdir, capture_output=True, text=True, timeout=timeout,
    )
    duration = time.time() - start

    parsed = _parse_roko_output(proc.stdout)
    diff = _git_diff(workdir)

    return {
        "model_patch": diff,
        "cost": parsed.get("cost_usd", 0.0),
        "tokens": parsed.get("input_tokens", 0) + parsed.get("output_tokens", 0),
        "duration_s": duration,
        "model": parsed.get("model", model),
        "workflow": workflow,
        "exit_code": proc.returncode,
    }


def _setup_workspace(task: dict[str, Any]) -> str:
    """Clone the task's repo at the base commit; init roko."""
    if "repo" not in task:
        return task.get("workdir", os.getcwd())

    workdir = tempfile.mkdtemp(prefix="roko-hal-")
    subprocess.run(
        ["git", "clone", "--depth", "50", task["repo"], workdir],
        check=True, capture_output=True,
    )
    if "base_commit" in task:
        subprocess.run(
            ["git", "checkout", task["base_commit"]],
            cwd=workdir, check=True, capture_output=True,
        )
    # Initialise roko if not already present.
    if not (Path(workdir) / "roko.toml").exists():
        subprocess.run(
            ["roko", "init", "--non-interactive"],
            cwd=workdir, capture_output=True,
        )
    return workdir


def _build_prompt(task: dict[str, Any]) -> str:
    """Compose the user prompt from a HAL task dict."""
    parts: list[str] = []
    if "problem_statement" in task:
        parts.append(task["problem_statement"])
    elif "prompt" in task:
        parts.append(task["prompt"])
    elif "issue" in task:
        parts.append(f"Fix this issue:\n\n{task['issue']}")
    if "hints" in task:
        parts.append(f"\nHints:\n{task['hints']}")
    return "\n\n".join(parts) or "Analyze the workspace and propose changes."


def _parse_roko_output(stdout: str) -> dict[str, Any]:
    """Parse roko --output json. Tolerant of mixed stdout."""
    # The last JSON object on stdout is the canonical result.
    last_obj = None
    for line in stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"): continue
        try:
            last_obj = json.loads(line)
        except json.JSONDecodeError:
            continue
    return last_obj or {"raw_stdout": stdout[:1000]}


def _git_diff(workdir: str) -> str:
    """Get the diff produced by roko (HEAD -> working tree)."""
    proc = subprocess.run(
        ["git", "diff", "HEAD"], cwd=workdir, capture_output=True, text=True,
    )
    return proc.stdout if proc.returncode == 0 else ""
```

**`hal/roko_agent/requirements.txt`**:

```text
hal-harness>=0.4.0
```

**`hal/README.md`** — short guide:

```markdown
# Roko HAL Integration

This directory contains the wrapper that exposes the roko CLI as a HAL
agent.

## Quick start

```bash
pip install -r hal/roko_agent/requirements.txt
cargo build --release -p roko-cli

# Run roko on SWE-bench mini (50 tasks)
hal-eval \
  --benchmark swe_bench_verified_mini \
  --agent_dir hal/roko_agent/ \
  --agent_function main.run \
  --agent_name "roko-dev" \
  -A model_name=gpt-4.1-mini \
  -A workflow=standard \
  -A gates=express \
  -A roko_binary=$PWD/target/release/roko \
  --max_concurrent 5
```

Results land under `hal_results/`.
```

### Step 3 — CI workflow

`.github/workflows/hal-bench.yml`:

```yaml
name: HAL benchmark
on:
  schedule:
    - cron: '0 5 * * *'           # 05:00 UTC daily
  workflow_dispatch:

jobs:
  hal-bench:
    runs-on: ubuntu-latest
    timeout-minutes: 180
    env:
      OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-python@v5
        with: { python-version: '3.12' }
      - name: Build roko
        run: cargo build --release -p roko-cli
      - name: Install HAL
        run: pip install -r hal/roko_agent/requirements.txt
      - name: Run HAL (SWE-bench mini)
        run: |
          hal-eval \
            --benchmark swe_bench_verified_mini \
            --agent_dir hal/roko_agent/ \
            --agent_function main.run \
            --agent_name "roko-nightly-${{ github.sha }}" \
            -A model_name=gpt-4.1-mini \
            -A workflow=standard \
            -A gates=express \
            -A roko_binary=$PWD/target/release/roko \
            --max_concurrent 4
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: hal-results-${{ github.sha }}
          path: hal_results/
          retention-days: 90
```

---

## Step-by-step execution

1. `git checkout -b perf/17-hal-integration`.
2. Verify `--output json` works; add it if missing (Step 1).
3. Create `hal/roko_agent/{main.py,requirements.txt}` (Step 2).
4. Add `hal/README.md`.
5. Smoke-test locally:

   ```bash
   pip install hal-harness
   cargo build --release -p roko-cli
   # Run against a single task fixture (HAL ships small examples).
   hal-eval --benchmark swe_bench_verified_mini \
     --agent_dir hal/roko_agent/ --agent_function main.run \
     --agent_name "roko-smoke" \
     -A model_name=gpt-4.1-nano \
     -A roko_binary=$PWD/target/release/roko \
     --max_tasks 1
   ```

6. Add CI workflow (Step 3) but disable the `schedule` trigger until
   the smoke test passes from CI manually.
7. PR `feat(hal): expose roko as HAL agent + nightly CI`.

---

## Anti-patterns / things NOT to do

- **Do NOT run HAL on every PR.** Each task costs API spend; SWE-bench
  mini at 50 tasks is ~$5-10. Daily/weekly is the right cadence.
- **Do NOT clone the full repo** for every task. `--depth 50` is enough
  to satisfy `git diff` and `git checkout`.
- **Do NOT crash on `task` shape variations.** SWE-bench Verified
  mostly uses `problem_statement`, but other benchmarks use `prompt`,
  `issue`, etc. The `_build_prompt` helper handles all three.
- **Do NOT swallow non-zero exit codes silently.** Forward `exit_code`
  in the response so HAL can score correctly. A non-zero exit is a
  failed task, not a missing result.
- **Do NOT `roko init` if `roko.toml` already exists.** The check in
  `_setup_workspace` is essential — re-init can clobber test config.
- **Do NOT commit secrets.** The CI workflow uses `secrets.OPENAI_API_KEY`;
  do not hard-code keys in `main.py` or `requirements.txt`.
- **Do NOT print secrets in error messages.** roko's existing
  `--output json` should already redact API keys; verify in a test.
- **Do NOT cache `_setup_workspace`** results across tasks. Each task
  must run in a clean clone — diff bleed between tasks invalidates the
  benchmark.
- **Do NOT spawn `--gates full` in HAL.** Most tasks change source; the
  workspace's CI is unrelated to ours, and running `cargo test` on
  random GitHub repos is futile and slow. Use `--gates express` (or
  `--gates none` if even that fails) and let HAL's own tester apply
  the patch + run the upstream test suite.

---

## Test plan

| Level | Test | How |
|---|---|---|
| Unit (Python) | `_build_prompt` handles all task shapes | `pytest hal/roko_agent/tests/test_main.py` |
| Unit (Python) | `_parse_roko_output` is tolerant of mixed stdout | same |
| Smoke | Single-task HAL run completes (`--max_tasks 1`) | manual / CI |
| End-to-end | 50-task SWE-bench mini run produces a pass-rate | nightly CI |
| Regression | Pass rate does not drop >10 % across two consecutive runs | dashboard check |

---

## Rollback plan

- Disable the schedule trigger; everything else is additive.
- `git revert` removes the wrapper; no Rust code change.

---

## Status check (acceptance)

- [ ] `roko run --output json` produces valid JSON.
- [ ] `hal/roko_agent/main.py` and supporting files exist.
- [ ] Smoke test passes locally.
- [ ] CI workflow exists; first manual run succeeds.
- [ ] First nightly result posted to the team channel / leaderboard.
