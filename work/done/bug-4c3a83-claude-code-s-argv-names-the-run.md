+++
id = "bug-4c3a83"
kind = "bug"
title = "Claude Code's argv names the run and the benchmark: its inline --settings deny rules hold <results>/<experiment>/<run_id> and ~/.config/viabilitybench, and the agent's Bash tool reads its parent's argv with sysctl(KERN_PROCARGS2) inside the sandbox"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "f0395912d"
source = "bug-d308f7 worker (claude-agent), 2026-10-17 (run_cli.settings and build_invocation; on macOS 26.4 a child under common.sandbox.command reads its parent's argv, a --settings JSON naming .config/viabilitybench included, with ctypes sysctl KERN_PROCARGS2, rc 0)"
discovered_from = "bug-d308f7"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py::settings", "benchmarks/viabilitybench/driver/run_cli.py::build_invocation"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d308f7", "bug-3bab88", "bug-d68575", "bug-3c921e", "gap-b94a2c", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_cli_argv_names_the_run_or_the_benchmark' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_no_cli_argv_names_the_run_or_the_benchmark -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T14:06:24Z"
commit = "f0395912d"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T11:04:41Z"
forced = false
evidence = "f0395912d: Claude Code's command line, as the driver builds it for the fd_claude and fd_claude_lite arms and the roko_plan arm's planner, names neither the experiment, the run, the results root nor the benchmark. run_cli.build_invocation builds the sandbox first and passes run_cli.settings the paths of ctx.deny (the results root since bug-935cfa, the key files) only when no sandbox applies: on macOS the sandbox denies the whole session those paths, Claude's file tools included (claude itself runs under it), so the rules added only their paths to the argv that the session's Bash tool reads with sysctl(KERN_PROCARGS2). The default secret directory's rule is the operator's whole ~/.config, which names no benchmark, in place of ~/.config/viabilitybench. Under the sandbox, --settings deny the web tools, the repository, ~/.roko and ~/.config; without one they also name the results root, the only thing then keeping it from Claude's file tools. A --settings file would not have helped: the session can read any file claude reads. Decided under Will's decision 9 (## Decision below): no benchmark-internal identifier reaches an agent, for benchmark validity and against contamination through the public repository. The verify passes: test_no_cli_argv_names_the_run_or_the_benchmark (driver/test_run_cli.py, macOS), 1 passed: through vb run, the fake claude logs its argv and its Bash tool reads its parent's argv and environment with test_secret.py's PROCARGS from inside the sandbox; neither names TEST-CLI, run-1, the results root or the benchmark (BENCHMARK_MARK), the probe did read claude's --settings (the control), and the task completed under sandbox-exec+net. Before this change it fails on the argv's Edit(//<home>/.config/viabilitybench/**) and Read(/<tmp>/results/**). test_claude_arm_command_is_isolated_and_pinned still finds the results root in --settings when no sandbox applies. Every other arm's parent was checked (the item's step 2): Codex's command line holds its fixed flags, the workdir alias and the model; Roko's its binary under the repository, the workdir alias and the plan's neutral slug; fr_claude's claude is Roko's own, built from the emitted roko.toml. Bench pytest: 815 passed, 16 skipped (13 with no roko binary in the worktree, the analysis stack, two opt-in sweeps), against 814 passed and the same 16 skipped before. Filed bug-0fd744 (p3: the direct loop's shell is a child of the driver, whose command line and environment name the experiment, the run id, the benchmark's path and VB_DRIVER_ENV; a vb run started as test_secret.py starts it showed all of them to the direct loop's agent through PS_PROBE from inside the sandbox) and bug-e16450 (p3: mini-swe-agent runs from .venv-msa inside the benchmark tree, so its agent's parent's command line names benchmarks/viabilitybench)."
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

- [x] Claude Code's command line, as the driver builds it (the fd_claude and fd_claude_lite arms, the roko_plan arm's
  planner), carries neither the run id, the experiment id nor the benchmark's name; neither do the Codex and Roko
  arms' (2026-10-17: the two parents that still do are filed apart, see Notes).
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-d308f7 worker, 2026-10-17, while checking what an agent can learn of its run directory's path.
- Every other process of the operator's user stays readable the same way (`agent_env`, gap-0bd49a): gap-b94a2c owns
  that mechanism, which no sandbox-exec rule tried stops, and only a separate user or a container per task hides
  (S08 decision 4). This item is narrower: what the driver itself puts on its agents' parents' command lines.
- Fixed 2026-10-17 (option "make them neutral"). `run_cli.build_invocation` builds the sandbox first and passes
  `run_cli.settings` the paths of `ctx.deny` (the results root since bug-935cfa, the key files) only when no sandbox
  applies: on macOS the sandbox denies the whole session those paths, Claude's file tools included, so the rules
  would add nothing but the results root's path on argv. The default secret directory's rule is now the operator's
  whole `~/.config`, which names no benchmark. Claude Code's `--settings` under the sandbox: the web tools, the
  repository, `~/.roko` and `~/.config`. A file passed with `--settings <path>` would not have helped: the session
  can read any file claude can, and whether Claude Code 2.1.282 re-reads it could not be checked without a paid turn.
- Step 2, every arm's parent: Codex's command line holds its fixed flags, the workdir alias and the model; Roko's its
  binary (under the repository, which names no run or benchmark), the workdir alias and the plan's neutral slug; the
  fr_claude arm's claude is Roko's own, built from the emitted `roko.toml`. Two parents still name the benchmark:
  the direct loop's shell is a child of the driver, whose command line and environment name the experiment, the run
  id (a campaign's unit's) and the benchmark's path (bug-0fd744, checked from inside the sandbox), and
  mini-swe-agent runs from `.venv-msa/` inside the benchmark tree (bug-e16450).
- The original first box was "No agent's parent process carries the run id, the experiment id or the benchmark's
  name on its command line"; it now covers what this item changed, and the two items above hold the rest.

## Decision (2026-10-17)

Whether Claude Code's command line may carry the run's paths and the benchmark's name had no recorded default.
Under Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the coordinator's
recommendation, the conservative option, recorded on the item with the reasoning; Will can veto it later), the
coordinator decided that no benchmark-internal identifier reaches an agent: the command line the driver gives an
agent's parent names neither the experiment, the run, the results root nor the benchmark. The change lands before
the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. The session's Bash tool reads claude's
argv from inside the sandbox, and the run directory's path named the experiment and the run id (a campaign's names
its block, `e-oss-1` the honeypots), and `~/.config/viabilitybench` named the benchmark, a search key into the
public repository with its driver, generators and truth suites. Neutral rules are the conservative option: under the
sandbox the session is denied exactly what it was (the sandbox denies the results root and the key files to every
process of the session, Claude's file tools included), Claude's file tools are denied more of `~/.config` than
before and nothing else changes, so nothing an agent needs or is graded on changes, only what it can read.
