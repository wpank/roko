+++
id = "bug-4c3a83"
kind = "bug"
title = "Claude Code's argv names the run and the benchmark: its inline --settings deny rules hold <results>/<experiment>/<run_id> and ~/.config/viabilitybench, and the agent's Bash tool reads its parent's argv with sysctl(KERN_PROCARGS2) inside the sandbox"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-d308f7 worker (claude-agent), 2026-10-17 (run_cli.settings and build_invocation; on macOS 26.4 a child under common.sandbox.command reads its parent's argv, a --settings JSON naming .config/viabilitybench included, with ctypes sysctl KERN_PROCARGS2, rc 0)"
discovered_from = "bug-d308f7"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py::settings", "benchmarks/viabilitybench/driver/run_cli.py::build_invocation"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d308f7", "bug-3bab88", "bug-d68575", "bug-3c921e", "gap-b94a2c", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_cli_argv_names_the_run_or_the_benchmark' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_no_cli_argv_names_the_run_or_the_benchmark -q"
+++

## Problem

The Claude Code arm passes its deny rules inline, `--settings '{"disableAllHooks":true,"permissions":{"deny":[...]}}'`
(`run_cli.build_invocation`, `run_cli.settings`), so claude's command line names, as absolute paths: the repository,
the operator's `~/.roko`, the run directory `<results>/<experiment>/<run_id>` (by default under
`~/.roko-bench/viability`) and the default secret directory `~/.config/viabilitybench`. The session's Bash tool runs
under claude, and from inside the agents' sandbox `sysctl(KERN_PROCARGS2)` returns another process's argv (and
environment) for the same user: the setuid `ps` cannot start there, the sysctl can (`agent_env`'s docstring notes the
same for environments). So an agent that reads its parent's argv learns the experiment id, the run id (a campaign's
names its block: `e-oss-1` is LOG1's block E, the honeypots), the benchmark's name and where its results live.

To see it (checked 2026-10-17 on macOS 26.4): a parent whose argv holds `--settings` with
`Read(//Users/x/.config/viabilitybench/**)` starts a child through `common.sandbox.command(..., network="none")`; the
child's `sysctl([CTL_KERN, KERN_PROCARGS2, getppid()])` (ctypes, no setuid) returns rc 0 and the parent's argv, the
`--settings` JSON included. The roko_plan arm's planner runs `run_cli.build_invocation` too.

## Why it matters

Goal `proof`. Under Will's decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier
reaches an agent (bug-3c921e's Decision): neither its run id nor the benchmark's name (bug-d68575). The run
directory's path also tells an agent where to probe (the existence oracle, bug-3bab88). Before the
pre-registration lock (gap-394f28), since it changes what the CLI arms' agents can learn.

## Where

- `benchmarks/viabilitybench/driver/run_cli.py::settings`: the deny rules, built from `layout.REPO_ROOT`,
  `~/.roko`, the run directory and `~/.config/viabilitybench`.
- `benchmarks/viabilitybench/driver/run_cli.py::build_invocation`: `--settings` on argv.

## Current state

The rules came with the fd_claude arm (S08 §4.9, "where the CLI supports them"). Since bug-d308f7 the sandbox denies
the whole session the run directory on macOS, so the rule for it binds only off macOS, where nothing applies the
sandbox; the path is still on argv.

## Plan

1. Write the settings to a file the session can read but whose path and content name none of these (for example deny
   the operator's whole `~/.config` and `~/.roko-bench`-style roots by a neutral parent, or name the run directory
   through a symlink with a random name), or pass rules that need no absolute path; check what Claude Code 2.1.282
   accepts (`--settings <file>`).
2. Check the other arms' argv the same way: Codex's `-c` flags, Roko's `--repo`, mini-swe-agent's `-c`/`-o` (paths
   in a driver temp directory).
3. Add `test_no_cli_argv_names_the_run_or_the_benchmark` to `driver/test_run_cli.py`: the fake claude logs its argv
   (`FAKE_CLAUDE` already does); it names neither the experiment id, the run id, nor `viabilitybench`.

## Done when

- [ ] No agent's parent process carries the run id, the experiment id or the benchmark's name on its command line.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-d308f7 worker, 2026-10-17, while checking what an agent can learn of its run directory's path.
- Every other process of the operator's user stays readable the same way (`agent_env`, gap-0bd49a): gap-b94a2c owns
  that mechanism, which no sandbox-exec rule tried stops, and only a separate user or a container per task hides
  (S08 decision 4). This item is narrower: what the driver itself puts on its agents' parents' command lines.
