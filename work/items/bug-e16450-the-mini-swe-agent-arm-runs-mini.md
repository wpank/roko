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
updated = 2026-10-17
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

- [ ] The mini-swe-agent arm's agent's parent process carries neither the benchmark's name nor its path on its
  command line.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-4c3a83 worker, 2026-10-17, while checking every arm's parent command line (that item's plan,
  step 2).
- The Roko arm's binary, `<repo>/target/debug/roko`, names the repository but not the benchmark; Codex's and Claude
  Code's are installed outside it.
