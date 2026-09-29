+++
id = "gap-28ebea"
kind = "gap"
title = "ViabilityBench driver for the direct arm (S08.T6)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§4.9–4.10, §5.4, §6 T6; checklist S08.T6)"
anchors = ["benchmarks/viabilitybench/driver/", "benchmarks/viabilitybench/arms/cheap_direct.toml", "benchmarks/viabilitybench/arms/fd_api.toml", "benchmarks/viabilitybench/streams/pilot.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-2790c5"], blocks = [], related = ["gap-33d54b", "gap-a8a160"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_runaway_agent_is_killed_within_30_calls' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_runaway_agent_is_killed_within_30_calls -q"

[[verify]]
command = "grep -qw 'def test_offline_run_writes_valid_records' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_offline_run_writes_valid_records -q"
+++

## Problem

Nothing can run an agent on a benchmark task and label the result honestly: `roko bench` scores the agent's own
output (S08 D1) and falls back to simulation (D8). The cheap-model-alone arm needs a driver that:

- materializes each task and runs a bash-only tool loop on gpt-oss-120b under hard caps;
- archives the final tree and labels it by VS census;
- writes `vb.run_record/1` rows priced from the snapshot.

## Why it matters

Every other arm plugs into this driver. It enforces S08 SC6 (the caps bind) and SC7 (the records are honest). One
gpt-oss run once made 1,109 calls (S09 §3), so the caps must be in place before any paid call.

## Where

All new, and the paths follow D4:

- `benchmarks/viabilitybench/driver/`: `vb.py`, `materialize.py`, `mini_loop.py`, `caps.py`, `ledger.py`,
  `archive.py`, `census.py`, `records.py`, `agent_env.py`, `test_driver.py`, and a toy family in `testdata/`;
- `arms/cheap_direct.toml`, `arms/fd_api.toml` and `streams/pilot.toml`.

## Current state

Checked at `41c7ffbd6`: nothing exists. `scripts/dev_benchmark.py::execute` already refuses provider runs without
both `--allow-network` and an explicit `--max-cost-usd`; reuse that admission rather than build a fourth harness
(W10 rec 14).

## Plan

1. **Dispatch.** `vb run --experiment X --stream pilot --arm cheap_direct --model gpt-oss-120b --seeds 1-3`
   chooses a runner module from the `harness` field of the arm's file. Later arms then add files without editing
   `vb.py`.
2. **Workdirs.** Every (task, seed) gets a fresh workdir under `$VB_WORK`, outside the repo.
3. **Agent environment.** Every agent process gets an allowlisted environment from `agent_env.py`, with no `VB_*`
   variables and no provider keys.
4. **Caps** (`mini_loop.py`, `caps.py`).
   - Per attempt: 150K input tokens and 12 turns.
   - Per task: 30 turns, 300K input tokens and 20 minutes.
   - A runaway detector kills the task at 30 model calls, at 5 identical tool calls, or at its USD cap. The status
     becomes `aborted_cap`, and VS = 0.
5. **Ledger** (`ledger.py`). Append-only rows, priced from `prices-2026-09-28`. An unknown model's cost is `null`.
   Budget lines come later, in gap-33d54b.
6. **Archive and census.** `archive.py` saves a git bundle and a tarball of the final tree. `census.py` re-runs the
   visible checks on a clean worktree with the test files restored, runs the hidden suite with `--secret-file` and
   the integrity checks, and counts canary hits. `records.py` writes `simulated: false`.
7. **Network admission** as in `dev_benchmark.py`: no provider call without both flags.

## Done when

- [ ] An offline run of two toy tasks, against a scripted fake model on a local stub server, writes schema-valid
      records with census labels.
- [ ] A synthetic runaway agent is killed within 30 calls.
- [ ] Without the two admission flags, `vb run` exits before any network call.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **This item spends nothing.** S08's 20 × 3 paid acceptance runs move to Pilot A (gap-c33709), which breaks the
  checklist's circular `budget` gate (W10 rec 2).
- **No wait for F1 or F4:** the tests use the toy family. mini-swe-agent (S08 decision 3) is out of scope.
- **If it runs long** (S08 sizes it M–L), split the archive and census into their own item. No hot files.
