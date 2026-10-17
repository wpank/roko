+++
id = "bug-d68575"
kind = "bug"
title = "Every agent's environment names the benchmark: PATH's .vb-bin, git's vb-agent <agent@vb.invalid>, the pristine commit 'vb: pristine base', the direct loop's VB_SUBMIT, and in flaky_verify runs a wrapper whose own source explains the flakes"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-3c921e worker (claude-agent), 2026-10-17 (agent_env.FIXED and build, common/repo.py::init_task_repo, mini_loop.SUBMIT, run_roko.OFFLINE_KEY, vb_verify.install; test_an_agent_never_sees_its_instance_id's env and HOME listings)"
discovered_from = "bug-3c921e"
anchors = ["benchmarks/viabilitybench/driver/agent_env.py::FIXED", "benchmarks/viabilitybench/driver/agent_env.py::build", "benchmarks/viabilitybench/families/common/repo.py::init_task_repo", "benchmarks/viabilitybench/driver/mini_loop.py::SUBMIT", "benchmarks/viabilitybench/driver/vb_verify.py::install"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-3c921e", "bug-b5eab4", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_agent_sees_the_benchmark_name' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_no_agent_sees_the_benchmark_name -q"
+++

## Problem

What every agent sees carries the benchmark's own abbreviation, "vb":

- PATH starts with `<HOME>/.vb-bin` (`agent_env.build`), in every arm;
- git's identity is `vb-agent <agent@vb.invalid>` (`agent_env.FIXED`), in every arm, in `env` and on any commit the
  agent makes;
- the pristine base commit's message is `vb: pristine base` (`common/repo.py::init_task_repo`), in every arm's
  `git log`;
- the direct loop's submit command is `echo VB_SUBMIT` (`mini_loop.SUBMIT`), in its system prompt;
- the Roko arms' environment holds the placeholder key `vb-offline-placeholder` (`run_roko.OFFLINE_KEY`), and their
  plan is `plans/vb-<hash>/` (`planemit.plan_slug`);
- in a run with flaky_verify, every task's `.vb-bin/` holds `vb-verify`, `vb-verify.json` and a copy of
  `driver/vb_verify.py`, whose docstring explains the disturbance: S08 §4.6, `VB_FLAKE_P`, how a visible check is
  killed at random, and "The agent is not told";
- the default work root is `~/vb-work`, in every path (an operator can pass `--work`).

`agent@vb.invalid` and `VB_SUBMIT`, for two, are search keys into the public repository, where the driver, the
families' generators and their truth suites are.

## Why it matters

Goal `proof`. bug-b5eab4 renamed the families' S08 names because a name an agent can search for leads it to the hidden
suites and to its family. These marks are the same for every family (not like bug-d7ff47's ids, which marked four
families of eight), but they tell every agent that it is in a benchmark and where its code lives, and in a run with
flaky_verify the wrapper's own source tells it that its visible checks may be killed at random, which the
disturbance is meant to hide. Under Will's decision 9 the coordinator decided on 2026-10-17 that no
benchmark-internal identifier reaches an agent (bug-3c921e's Decision). `VB_SUBMIT` is in the direct arm's prompt
hash and the commit message is in every pristine commit, so a change belongs before the pre-registration lock
(gap-394f28).

## Where

- `benchmarks/viabilitybench/driver/agent_env.py::FIXED` (git identity) and `::build` (`.vb-bin`).
- `benchmarks/viabilitybench/families/common/repo.py::init_task_repo` (the commit message; `export_tree`'s too).
- `benchmarks/viabilitybench/driver/mini_loop.py::SUBMIT`, `benchmarks/viabilitybench/driver/run_roko.py::OFFLINE_KEY`,
  `benchmarks/viabilitybench/driver/planemit.py::plan_slug`.
- `benchmarks/viabilitybench/driver/vb_verify.py::install` (the launcher, its config and the copied module).
- `benchmarks/viabilitybench/driver/vb.py::DEFAULT_WORK`.

## Current state

Since the driver's first version. bug-3c921e gave the workdir, HOME and Roko's plan random names under these
prefixes.

## Plan

1. Pick neutral names: for example `.local/bin`, `agent <agent@example.invalid>`, "initial commit", a submit word
   that names no benchmark, a neutral placeholder and slug prefix, and a neutral default work root.
2. Install the wrapper under a neutral name, with a copy of the module stripped of its docstring.
3. A new commit message changes every pristine commit hash, and a new submit word the direct arm's prompt: bump
   `mini_loop.PROMPT_VERSION` and the generators that need it, and note it for the lock.
4. Add `test_no_agent_sees_the_benchmark_name` to `driver/test_driver.py`: an offline run with flaky_verify whose agent
   prints its env, `git log` and its HOME's files; no observation, and no file under the kept work directory,
   matches `\bvb\b`, `vb[-_]`, `[-_.]vb\b` or `viabilitybench` (case-insensitive).

## Done when

- [ ] Nothing an agent sees names the benchmark.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-3c921e worker, 2026-10-17, while checking what else in an agent's environment names its task.
- The generators and truth suites are public by design (`common/hmac_seed`: an agent may read them, but not compute
  the hidden cases without the secret). These marks only make them, and the family an instance belongs to, easy to
  find.
