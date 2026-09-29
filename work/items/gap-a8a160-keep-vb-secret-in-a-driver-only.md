+++
id = "gap-a8a160"
kind = "gap"
title = "Keep VB_SECRET in a driver-only file and prove agents cannot read it"
status = "open"
triage = "verified"
last_verified = 2026-09-29
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W10-benchmarks-proof.md (verified leak, rec 3); specs/S08-benchmark-suite.md (§9 decision 8)"
anchors = ["benchmarks/viabilitybench/driver/secret.py", "benchmarks/viabilitybench/driver/test_secret.py", "benchmarks/viabilitybench/driver/census.py", "benchmarks/viabilitybench/driver/agent_env.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-28ebea"], blocks = [], related = ["bug-7d7200", "gap-2790c5", "gap-7ee7c2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_secret_never_reaches_an_agent_env' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_secret_never_reaches_an_agent_env -q"

[[verify]]
command = "grep -qw 'def test_run_refused_when_secret_is_in_roko_dotenv' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_run_refused_when_secret_is_in_roko_dotenv -q"
+++

## Problem

By default, S08 §9 (decision 8) keeps the hidden-test secret `VB_SECRET` in `~/.roko/.env`. That file is not
private to the driver:

- `roko` loads it into its own process at startup (`load_startup_env_files`, `crates/roko-cli/src/main.rs:4499`);
- agents and verify commands then inherit roko's whole environment (bug-7d7200).

So every agent in the Roko arm could read the secret and derive the hidden cases. S08 SC4 requires the secret to
reach no agent's environment, and this default fails it by construction (W10).

## Why it matters

The hidden tests are what make the VS label honest, and a leaked secret makes every label worthless. Pilot A
depends on this item.

## Where

- **New:** `benchmarks/viabilitybench/driver/secret.py` and `driver/test_secret.py`. The paths follow D4.
- **Changed:** `driver/census.py` and `driver/agent_env.py`, both from gap-28ebea.

## Current state

Checked at `41c7ffbd6`:

- `main.rs:4499` still loads `~/.roko/.env`.
- The fix for bug-7d7200 (`dc99a9e81`, on branch `fix/hermetic-child-env`) is not merged.
- None of the benchmark exists yet.

## Plan

1. **A driver-only file.**
   - Keep the secret in a file with mode 0600, inside a directory with mode 0700.
   - The directory is outside the repo, every workdir and `~/.roko`: for example `~/.config/viabilitybench/secret`,
     which `VB_SECRET_FILE` overrides.
   - The file also holds a canary line of its own.
2. **Read it only at census time.** The driver reads the secret after the agent's process has exited, and passes it
   to `hidden.py` as `--secret-file`: never in an environment variable, never on a command line.
3. **Fail closed.** `vb run` refuses to start if `VB_SECRET` is set in its own environment, or appears in
   `~/.roko/.env` or in the workspace's `.roko/.env`.
4. **One environment builder.** Every agent launch takes its environment from `agent_env.py`, which asserts that the
   secret is absent. That covers the direct loop's shell, `roko` and `claude`.
5. **Proof.**
   - Plant a step that dumps the environment, in the direct loop and in a Roko-arm verify command. Run the Roko one
     once against the built `roko`. Neither output may contain the secret or its canary.
   - The census adds the file's canary to the canaries it looks for in transcripts and diffs.

## Done when

- [x] Both tests pass, and the closing evidence records the one-off Roko-arm environment dump.
- [x] Both `[[verify]]` commands pass.

## Notes

- **What 0600 does and does not do.** It stops other users, not an agent that runs under the same uid. What it
  prevents is inheritance and accidental exposure. If an agent deliberately reads the file, a canary hit marks the
  run `leak_suspected`.
- **A stronger option.** A container per task (S08 decision 4) is stronger, and out of scope here.
- **No hot files.**
- **Done 2026-09-29 (wk-bench-secret).** `driver/secret.py` owns the file (`init`/`check`, 0600 in a 0700 directory,
  its own canary line) and `vb run`'s fail-closed `preflight`; `agent_env.forbid` makes every agent environment refuse
  the secret and its canary; the census refuses a leaky census environment, looks for the secret (and for commands
  naming its file) in the transcript, diff and tree, sweeps what agent code could leave during the census, and redacts
  what it reports. Adversarial fake agents (env, file search from the workdir, `ps -E` and `/proc` on the driver and on
  `hidden.py`) are in `driver/test_secret.py`.
- **One-off Roko-arm environment dump (2026-09-29)**, built `roko` from the main checkout (`target/debug/roko`), the
  fake Claude CLI, no model call; a verify step ran `env | sort` and `cat ~/.roko/.env ~/.config/viabilitybench/secret`,
  and a wrapper dumped the agent CLI's environment:
  - before (S08 decision 8's layout, `VB_SECRET` in the operator's `~/.roko/.env`, roko started with that HOME):
    neither environment held the secret (roko no longer passes dotenv values to children, bug-7d7200), but both ran
    with the operator's HOME, and the verify step read the secret through `~/.roko/.env`;
  - after (the secret only in its file, `secret.preflight`, roko started with `agent_env.build`): verify environment
    13 names and agent environment 24 names, per-task HOME, no `VB_` name, no secret, no canary, nothing reachable
    through HOME.
