+++
id = "bug-d68575"
kind = "bug"
title = "Every agent's environment names the benchmark: PATH's .vb-bin, git's vb-agent <agent@vb.invalid>, the pristine commit 'vb: pristine base', the direct loop's VB_SUBMIT, and in flaky_verify runs a wrapper whose own source explains the flakes"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "d5fea59c9"
source = "bug-3c921e worker (claude-agent), 2026-10-17 (agent_env.FIXED and build, common/repo.py::init_task_repo, mini_loop.SUBMIT, run_roko.OFFLINE_KEY, vb_verify.install; test_an_agent_never_sees_its_instance_id's env and HOME listings)"
discovered_from = "bug-3c921e"
anchors = ["benchmarks/viabilitybench/driver/agent_env.py::FIXED", "benchmarks/viabilitybench/driver/agent_env.py::build", "benchmarks/viabilitybench/families/common/repo.py::init_task_repo", "benchmarks/viabilitybench/driver/mini_loop.py::SUBMIT", "benchmarks/viabilitybench/driver/vb_verify.py::install"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-3c921e", "bug-b5eab4", "gap-394f28", "bug-4c3a83", "bug-fd8200"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_agent_sees_the_benchmark_name' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_no_agent_sees_the_benchmark_name -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T10:58:01Z"
commit = "d5fea59c9"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-17T08:24:30Z"
forced = false
evidence = "d5fea59c9: nothing an agent sees names the benchmark. PATH's bin directory is <HOME>/.local/bin (agent_env.BIN_DIR, which run_cli's keychain wrapper, run_codex's interpreter link and census.KEYCHAIN_RE follow), not .vb-bin; git's identity agent <agent@example.invalid>, not vb-agent <agent@vb.invalid> (USER and LOGNAME fall back to agent); the pristine base maintainer <maintainer@example.invalid> with 'Initial commit', not ViabilityBench <vb@localhost> with 'vb: pristine base' (that identity was also in plain text in every task repo's reflog), exports 'Export of ...', and init_task_repo no longer leaves an empty .git/refs/vb/ (the bundles keep refs/vb/pristine, which archive.py and the audit battery read); the direct loop's submit word SUBMIT_TASK, not VB_SUBMIT (mini_loop.PROMPT_VERSION mini-loop-2; PROMPT_SHA256 now covers the word); the key placeholder 'placeholder', not vb-offline-placeholder (run_roko, run_msa); Roko's plan plan-<hash>, not vb-<hash> (planemit-6, planemit-ladder-3); mini-swe-agent's scratch mini-*, not vb-msa-* (its MSWEA_GLOBAL_CONFIG_DIR and argv name it); the default work root ~/workspaces, not ~/vb-work; the visible-verify wrapper installed as run-shell, run-shell.json, run-shell.jsonl and run_shell.py, the copy being vb_verify.stripped_source(), the module without its docstrings and comments by an AST round trip (in a flaky_verify plan Roko's prompt showed 'vb-verify <check>'); the egress proxy's answers 'egress proxy: ... is not on the allowlist', not 'vb egress: ... this run's allowlist'; the metering proxy's error types the refusal's own name, not vb_*. Decided under Will's decision 9 (## Decision below): no benchmark-internal identifier reaches an agent, for benchmark validity and against contamination through the public repository. The verify passes: test_no_agent_sees_the_benchmark_name (driver/test_driver.py), 3 passed (direct loop, Roko and mini-swe-agent arms): an offline flaky_verify run through --proxy with --keep-workdirs, whose agent prints env, git log with identities, reflog and refs, every file of its HOME, Roko's plan and config, mini-swe-agent's config and the proxy's answer to a refused POST; nothing seen, and no name, link or text file under the work directory, matches \\bvb\\b, vb[-_], [-_.]vb\\b or viabilitybench (the host's temporary directories aside), and the wrapper's copy has no docstring or comment. test_no_cli_agent_sees_the_benchmark_name (driver/test_run_cli.py), 1 passed: the same for Claude Code's environment, prompt, Bash tool and HOME, and the egress proxy's answer. Each old mark put back alone makes them fail where it shows: the bin directory, the agent identity, the wrapper's names and its full source on all four tests; the pristine identity, the pristine message and the leftover refs/vb/ (repo.py edited, then restored) on all four; VB_SUBMIT on the direct loop; Roko's placeholder and slug on Roko; mini-swe-agent's placeholder and scratch prefix on mini-swe-agent; the vb_ error types on Roko and mini-swe-agent; the egress text on Claude Code. The instrument behaves as before: 116 renders of every family's task repo (every family and latent at levels 1-5 and seeds 1-2, and the plan slice's features) with the previous repo.py and with this one have the same files, modes, tree and commit tree, and differ only in the commit's author, committer and message, the reflog's identity and refs/vb/; task.json is unchanged, pristine.json and feature.json differ only in the commit; no task repo file or spec (61 renders, 1080 files) carries a mark. ci/verify_verifiers.py --levels 1-5 --seeds 2: 112 of 112 cells green (f1 20/20, f2 10/10, f3 10/10, f4 20/20, f5 10/10, f6 20/20, f8 10/10, pl 12/12); f7, whose checks need cargo, was not run. Bench pytest: 809 passed, 16 skipped (13 with no roko binary in the worktree, the analysis stack, two opt-in sweeps). Filed bug-fd8200 (p2: the metering proxy's cap refusals name the task key and its injected faults say 'injected'). Claude Code's argv still names ~/.config/viabilitybench in its --settings (bug-4c3a83, filed with bug-d308f7)."
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

- [x] Nothing an agent sees names the benchmark.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-3c921e worker, 2026-10-17, while checking what else in an agent's environment names its task.
- The generators and truth suites are public by design (`common/hmac_seed`: an agent may read them, but not compute
  the hidden cases without the secret). These marks only make them, and the family an instance belongs to, easy to
  find.
- More marks than the Problem lists, found on the way (2026-10-17) and made neutral too: the pristine commit's
  author and committer, `ViabilityBench <vb@localhost>` (`repo.GIT_IDENTITY`), shown by `git log` and kept in plain
  text in the reflog; the empty `.git/refs/vb/` that the bundle's temporary ref left in every task repo; the
  keychain wrapper in the bin directory (`run_cli`, and `census.KEYCHAIN_RE`, which names its path); Codex's
  interpreter link; mini-swe-agent's scratch directory `vb-msa-*`, which its `MSWEA_GLOBAL_CONFIG_DIR` and argv name,
  and the key placeholder in its config; the Roko arm's verify command in a flaky_verify run, `vb-verify '<check>'`,
  which Roko renders into its prompt; and the proxies' answers, `vb egress: ... is not on this run's allowlist` and
  the metering proxy's `vb_*` error types.
- Left as they are: what only the driver and the operator see (record labels such as `vb-keychain`, schema ids,
  driver-side temporary directories, the bundles' `refs/vb/pristine`, the proxy's `/_vb/control` path, which only
  its token opens). Claude Code's argv still names `~/.config/viabilitybench` and the run directory in its
  `--settings` (bug-4c3a83), and the metering proxy's messages still name the task key and say a fault was injected
  (bug-fd8200).
- The wrapper's copy keeps no docstring or comment, but its code still shows what it does, and its config and log
  (`p`, `seed`, `flake`) are readable, as any file of the agent's own is: only a wrapper out of the agent's reach
  hides that (S08 decision 4).
- Versions: `mini_loop.PROMPT_VERSION` is `mini-loop-2` and its `PROMPT_SHA256` now covers the submit word, which
  the templates only named as a placeholder; `planemit.TEMPLATE_VERSION` is `planemit-6` and `LADDER_VERSION`
  `planemit-ladder-3` (the slug, and the wrapper's name in a flaky_verify plan). No generator needs a bump: no test,
  fixture or lock pins a pristine commit, every task repo's files and tree are unchanged, and the suite hash, which
  covers `families/common/`, changes with `repo.py`. The default work root moves from `~/vb-work` to `~/workspaces`;
  `$VB_WORK` and `--work` override it as before.

## Decision (2026-10-17)

Whether what every agent sees may carry the benchmark's own abbreviation or name had no recorded default. Under
Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the coordinator's
recommendation, the conservative option, recorded on the item with the reasoning; Will can veto it later), the
coordinator decided that no benchmark-internal identifier reaches an agent: its PATH, its git identity, its repo's
commits, the direct loop's submit word, the Roko arms' plan and key placeholder, the visible-verify wrapper's files
and the proxies' answers name no benchmark. The change lands before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. These marks were the same for every
family, but they told every agent that it is in a benchmark, `agent@vb.invalid`, `VB_SUBMIT` and `ViabilityBench`
are search keys that lead to the driver, the generators and the truth suites, and in a flaky_verify run the wrapper's
own source explained that its visible checks may be killed at random, which the disturbance is meant to hide. Neutral
names are the conservative option: they change no task file, spec, check or caps, nothing an agent is graded on, and
the instrument behaves as before (the same files and trees, the same wrapper code, the same detection), only under
other names.
