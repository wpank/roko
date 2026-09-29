+++
id = "gap-8c3752"
kind = "gap"
title = "The census runs hidden.py, which executes agent code, without the sandbox agents get"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-bench-f4's report on gap-9e7079)"
anchors = ["benchmarks/viabilitybench/driver/census.py", "benchmarks/viabilitybench/families/f4_kvtool/hidden.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-a8a160", "gap-9e7079", "gap-4723ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_hidden_suite_runs_agent_code_without_the_secret_file' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_hidden_suite_runs_agent_code_without_the_secret_file -q"
+++

## Problem

F4's truth suite (`families/f4_kvtool/hidden.py`) runs the agent's migration script (`sh scripts/migrate_prefix.sh`) on each hidden store. It scrubs the environment (HOME and TMPDIR inside a temporary directory, a minimal PATH), and nothing else: the script runs as the driver's user, with the driver's filesystem. The census (`driver/census.py`, `_run_hidden`) starts `hidden.py` with `--secret-file` pointing at the HMAC secret, so at that moment the agent's code can read the secret file, the private `task.json` and the hidden stores. hidden.py's docstring says it: "the census must run hidden.py with the isolation it gives the agent". S08 decision 4 names a container per task as the strong form. F1's truth suite (on `work/gap-4723ff`) also runs agent code, in probe processes.

## Why it matters

Pilot benchmark (epic spec-567e52): a script that reads the secret at census time can derive the hidden cases of later runs, or tamper with the verdict's inputs. gap-a8a160 keeps the secret away from agents while they run, and leaves containers out of scope; the census window is not covered.

## Where

- `benchmarks/viabilitybench/driver/census.py`: `_run_hidden`, called at :100.
- `benchmarks/viabilitybench/families/f4_kvtool/hidden.py`: the migration run (about :170-180).

## Current state

At BASE no driver file uses a sandbox: no `sandbox-exec`, `bwrap` or container. `agent_env.py` scrubs the environment of agent launches only.

## Plan

1. Separate trusted from untrusted work in the truth suites. The family code that holds the secret derives the hidden stores first; the agent's code then runs in a sandbox that can't read the secret file, the private task directory or the driver's home: `sandbox-exec` with a deny-by-default profile on macOS, `bwrap` on Linux, or the per-task container of S08 decision 4.
2. Put F1's probe processes behind the same wrapper once F1 merges.
3. Record the sandbox kind in the census output.
4. Add `test_hidden_suite_runs_agent_code_without_the_secret_file`: a planted migration script tries to read the secret file and writes what it got into the store, and the census must show the read failed.

## Done when

- [ ] Agent code run by any truth suite can't read the secret file or the private task directory.
- [ ] The `[[verify]]` command passes.
