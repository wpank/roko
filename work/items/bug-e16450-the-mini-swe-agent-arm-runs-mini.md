+++
id = "bug-e16450"
kind = "bug"
title = "The mini-swe-agent arm runs mini from .venv-msa inside the benchmark tree, so the process its agent's commands run under has <repo>/benchmarks/viabilitybench/.venv-msa/bin/python and bin/mini on its command line, which the agent reads from inside the sandbox"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-18
source = "bug-4c3a83 worker (claude-agent), 2026-10-17 (run_msa.DEFAULT_BINARY is .venv-msa/bin/mini under layout.VB_ROOT, a console script whose shebang names the venv's python; mini-swe-agent 2.4.6 runs each command in a child shell, and from inside the agents' sandbox sysctl(KERN_PROCARGS2, getppid()) returns the parent's argv, as checked for Claude Code on the same day)"
discovered_from = "bug-4c3a83"
anchors = ["benchmarks/viabilitybench/driver/run_msa.py::binary_path", "benchmarks/viabilitybench/driver/run_msa.py::run_task"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-4c3a83", "bug-d68575", "gap-b94a2c", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_msa_parent_names_the_benchmark' benchmarks/viabilitybench/driver/test_run_msa.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_msa.py -k test_no_msa_parent_names_the_benchmark -q"
+++

## Problem

The mini-swe-agent arm runs `mini` from its own venv inside the benchmark tree: `run_msa.DEFAULT_BINARY`,
`.venv-msa/bin/mini` relative to `layout.VB_ROOT`, so `<repo>/benchmarks/viabilitybench/.venv-msa/bin/mini`. That is
a Python script whose shebang names the venv's interpreter, so the process the agent's commands run under has the
command line `<repo>/benchmarks/viabilitybench/.venv-msa/bin/python <repo>/benchmarks/viabilitybench/.venv-msa/bin/mini
-m openai/<model> -t <spec> ...`. mini-swe-agent runs each command through a shell that is its child, and from inside
the agents' sandbox `sysctl(KERN_PROCARGS2, getppid())` returns that command line (the mechanism bug-4c3a83's item
describes for Claude Code). So the agent's parent names the benchmark's directory, the repository's path and where
the benchmark's own code is.

## Why it matters

Goal `proof`. Under Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier
reaches an agent (bug-3c921e's Decision), the benchmark's name included (bug-d68575), and its paths are search keys
into the public repository. bug-4c3a83 took the run and the benchmark off Claude Code's command line; the
mini-swe-agent arm's parent still names the benchmark. Before the pre-registration lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/run_msa.py::binary_path` and `DEFAULT_BINARY`: where `mini` is run from.
- `benchmarks/viabilitybench/driver/run_msa.py::run_task`: the argv.

## Current state

The `-c` config and `-o` trajectory paths are in a driver temp directory named `mini-*` (neutral since bug-d68575);
the binary's own path is not. A symlink with a neutral name would not hide it: the sandbox lets the agent read the
link.

## Plan

1. Install the mini-swe-agent venv outside the repository under a neutral name (for example
   `~/.local/share/<neutral>/venv`), make that the default `[msa] binary`, and refuse at preflight an arm binary
   whose path names the benchmark (`vb`, `viabilitybench`) or lies inside the repository.
2. Update the README's set-up steps and `requirements-msa.lock`'s instructions.
3. Add `test_no_msa_parent_names_the_benchmark` to `driver/test_run_msa.py`: the fake `mini` (as the arm file names
   it) logs its own argv and executable path; neither names `benchmarks/viabilitybench`, `vb` or the run.

## Done when

- [x] The mini-swe-agent arm's agent's parent process carries neither the benchmark's name nor its path on its
  command line.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-4c3a83 worker, 2026-10-17, while checking every arm's parent command line (that item's plan,
  step 2).
- The Roko arm's binary, `<repo>/target/debug/roko`, names the repository but not the benchmark; Codex's and Claude
  Code's are installed outside it.
- Fixed 2026-10-18 (the plan's three steps). The default venv is `~/.local/share/mini-swe-agent/venv`
  (`run_msa.DEFAULT_VENV`; `DEFAULT_BINARY` is its `bin/mini`), and the arm file's `[msa] binary` names it.
  `run_msa.binary_path` refuses a binary whose path, as written or with its links followed, lies inside the
  repository or names the benchmark (`run_msa.BENCHMARK_NAME`, the tests' `BENCHMARK_MARK`), one that is not an
  executable file, and one whose first line names an interpreter that does: the kernel puts that interpreter on the
  command line, and a framework build of Python also puts it in `__PYVENV_LAUNCHER__`, so a neutral symlink or copy of
  a `mini` that still runs the old venv's python is refused too. `run_msa.preflight` asks it before the first task,
  so `vb run` stops with exit 2 and makes nothing; a missing venv is refused the same way, where every task used to end
  `infra_error`.
- The README ("mini-swe-agent's venv") and `requirements-msa.in`'s header have the set-up: `python3 -m venv` at that
  path, then `pip install --require-hashes -r requirements-msa.lock`; the lock is unchanged. Checked 2026-10-18 on
  macOS 26.4 with a venv made that way under a neutral path in the worker's scratch directory: it installs
  mini-swe-agent 2.4.6 from the lock, and an offline `vb run` of the arm with that real `mini` against the stub
  completed (VS 1, `sandbox-exec+net`, three calls to the stub). Its agent's first command read `mini`'s argv and
  environment from inside the sandbox: `Python.app <venv>/bin/mini -m openai/gpt-oss-120b -t <spec> -y
  --exit-immediately -c <tmp>/mini-*/mini.yaml -l 0 -o <tmp>/mini-*/trajectory.json`, the agent's environment, the
  `MSWEA_*` settings and `__PYVENV_LAUNCHER__=<venv>/bin/python`, none naming the benchmark, the run or the
  repository.
- `vb._results_hold` names the default venv, so a results root that holds it is still refused, as it was while the
  venv lay in the repository (every agent process is denied the whole results root, bug-935cfa).
- The tests' fake `mini`s now run under the base interpreter, the one agents run: the test venv's python lies inside
  the repository, which `binary_path` refuses.
- mini-swe-agent run by hand without `MSWEA_GLOBAL_CONFIG_DIR` (`mini --help`, say) loads, and creates if missing,
  `~/Library/Application Support/mini-swe-agent/` on macOS; `run_msa` sets that variable for every agent run.

## Decision (2026-10-18, Will)

Approved: run mini-swe-agent from a venv outside the benchmark tree, so its path names nothing benchmark-internal. Land it before the pre-registration lock.

Where to put the venv and what to refuse had no recorded default, so under Will's decision 9 of 2026-10-15 (technical
questions with no recorded default take the coordinator's recommendation, the conservative option, recorded on the
item with the reasoning; Will can veto it later) the change follows the coordinator's rule of 2026-10-17 (bug-3c921e's
Decision): no benchmark-internal identifier reaches an agent.

Why: benchmark validity, and contamination through the public repository. From inside the sandbox this arm's agent
read a parent whose command line and `__PYVENV_LAUNCHER__` named `<repo>/benchmarks/viabilitybench/.venv-msa/`: the
benchmark's name and where its driver, generators and truth suites sit, a search key into the public repository. A
venv under the operator's home with the tool's own name is the conservative option: the same pinned packages, from
the same hash-checked lock, run the same way, so nothing an agent does or is graded on changes, only the path it can
read; and refusing a path inside the repository or with the benchmark's name, as written, resolved or as the
interpreter, keeps a neutral-looking symlink or an old `.venv-msa/` from bringing the name back. The tool's own name
tells an agent nothing it cannot already read from `mini`'s flags.
