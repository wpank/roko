# PERF_19: HAL agent wrapper + nightly CI

## Task

Expose roko as a **HAL (Holistic Agent Leaderboard)** agent via a thin
Python wrapper, ensure `roko run --output json` is **machine-parseable
on stdout**, add **pytest** coverage for parsing edge cases, document
quick-start in `hal/README.md`, and add **`.github/workflows/hal-bench.yml`**
with `schedule` + `workflow_dispatch` (API costs: not on every PR).

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_19](../ISSUE-TRACKER.md#perf_19)
- Plan: `tmp/solutions/perf/implementation/17-hal-integration.md`
- Research: `tmp/solutions/perf/HAL-BENCHMARK-INTEGRATION.md`,
  `tmp/solutions/perf/HAL-AND-AGENT-BENCHMARKS.md`
- Performance contract: **C-18**
- Priority: EX (external eval)
- Effort: 17–20 h
- Depends on: none
- Wave: 1–2

## Problem

External benchmarks expect `run(task: dict, **kwargs) -> dict` with a
stable schema. Today, stdout noise or multi-line output breaks JSON
parsing. HAL needs a subprocess wrapper that clones repos, runs roko,
collects `git diff HEAD`, and returns cost/tokens/duration.

## Exact Changes

### Step 1 — Rust: `--output json` hygiene

In `crates/roko-cli/src/output_format.rs` and/or `main.rs`:

1. When `--output json` (or equivalent flag name already in tree) is
   active, **all human-oriented logs / progress / tracing to user
   terminal** must go to **stderr**; **stdout** contains **only** the
   final JSON document (single object, optionally pretty-printed — pick
   one and document it; HAL prefers **one** parseable blob).
2. The JSON object must include at least the keys the plan lists
   (`success`, `exit_code`, `duration_ms`, `model`, `cost_usd`,
   `input_tokens`, `output_tokens`, `files_changed`, …) — align field
   names with existing `BenchResult` / telemetry types where possible.
3. Add or extend a **Rust integration test** or unit test that captures
   stdout/stderr from a **mock** run path if full integration is heavy;
   otherwise document manual verification in PR.

### Step 2 — Python: `hal/roko_agent/main.py`

Implement (adapt from plan §Step 2):

- `run(task: dict[str, Any], **kwargs: Any) -> dict[str, Any]` with kwargs:
  `model_name`, `workflow`, `gates`, `timeout`, `roko_binary`.
- `_setup_workspace`: `git clone --depth 50`; `git checkout base_commit`;
  **`roko init` only if `roko.toml` missing**; use `roko_binary` for init
  if the binary path is not `roko` on PATH.
- `_build_prompt`: handle `problem_statement`, `prompt`, `issue`, optional `hints`.
- `_parse_roko_output`: tolerant parser — if multiple JSON lines, take
  the **last** successfully parsed object whose line starts with `{` (per
  plan).
- `_git_diff`: `git diff HEAD` text as `model_patch` source of truth for HAL.

Return dict keys (exact strings):

- `model_patch`, `cost`, `tokens`, `duration_s`, `model`, `workflow`, `exit_code`

### Step 3 — Packaging

- `hal/roko_agent/requirements.txt`: `hal-harness>=0.4.0` (bump if repo
  standard differs; pin minimum as in plan).
- `hal/README.md`: copy quick-start from plan §Step 2 with correct paths.

### Step 4 — Pytest `hal/roko_agent/tests/test_main.py`

- Tests for `_build_prompt` variants (empty / hints / alternate keys).
- Tests for `_parse_roko_output` with noisy stdout + valid trailing JSON.
- No network in unit tests; no real API keys.

### Step 5 — CI `.github/workflows/hal-bench.yml`

Per plan §Step 3:

- `on: schedule` (cron) + `workflow_dispatch`.
- Build `cargo build --release -p roko-cli`.
- `pip install -r hal/roko_agent/requirements.txt`.
- `hal-eval ...` with `-A roko_binary=$PWD/target/release/roko`.
- `OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}` — **never** hard-code.
- Upload `hal_results/` artifact with retention.

## Write Scope

- `hal/roko_agent/main.py` (**new**)
- `hal/roko_agent/requirements.txt` (**new**)
- `hal/roko_agent/tests/test_main.py` (**new**)
- `hal/README.md` (**new**)
- `.github/workflows/hal-bench.yml` (**new**)
- `crates/roko-cli/src/output_format.rs`
- `crates/roko-cli/src/main.rs` (flag wiring / stderr routing)

## Read-Only Context

- https://github.com/princeton-pli/hal-harness (agent protocol — browser)
- `tmp/solutions/perf/implementation/17-hal-integration.md`

## Acceptance Criteria

- [ ] `roko run --output json` → stdout is exactly one JSON object; logs on stderr.
- [ ] `hal/roko_agent/main.py` implements `run(...)` + helpers as specified.
- [ ] `requirements.txt` + `hal/README.md` exist.
- [ ] Pytests pass (`pytest hal/roko_agent/tests`).
- [ ] Workflow exists with `schedule` + `workflow_dispatch`; uses `secrets.OPENAI_API_KEY`.
- [ ] Clone uses `--depth 50`; skip `roko init` when `roko.toml` exists.
- [ ] No secrets in repo files.
- [ ] Commit message trailer: `tracker: PERF_19 done <sha>`.

## Verify

```bash
rg -n 'output json|OutputFormat' crates/roko-cli/src/main.rs crates/roko-cli/src/output_format.rs
python -m pytest hal/roko_agent/tests/ -q   # post-merge
```

## Do NOT

- Do NOT run full HAL on every `pull_request` (cost).
- Do NOT full-clone giant repos without `--depth 50`.
- Do NOT swallow non-zero roko exit codes — surface in `exit_code`.
- Do NOT `roko init` over an existing `roko.toml`.
- Do NOT print API keys in error paths.
- Do NOT use `--gates full` in the default HAL kwargs (use `express` per plan).
- Do NOT compile or run tests during the agent batch (`00-RULES.md`).

## Tracker update

```
tracker: PERF_19 done <commit-sha>
```
