+++
id = "gap-0bd49a"
kind = "gap"
title = "Agent shells in every ViabilityBench arm can reach the network, so an agent can fetch the public repo's hidden suites"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "L"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix2's report)"
anchors = ["benchmarks/viabilitybench/driver/harness.py", "benchmarks/viabilitybench/driver/agent_env.py", "benchmarks/viabilitybench/driver/mini_loop.py", "benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-f253cf", "gap-8c3752", "gap-308373"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_shell_cannot_reach_the_network' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_agent_shell_cannot_reach_the_network -q"
+++

## Problem

The driver gates network access for provider endpoints only (`vb.py`'s network admission, `--allow-network`). The agents' own shells get no network sandbox. Nothing in `harness.py`, `agent_env.py` or `mini_loop.py` uses `sandbox-exec`, `unshare`, firejail or a container.

So in every arm, an agent can:

- `curl` any URL;
- `git clone` the repository, which will be public and contains `families/*/hidden.py` and the plan-slice suites.

gap-f253cf removed fd_claude's web tools, but a shell command does the same thing.

## Why it matters

Pilot benchmark (epic spec-567e52), p1 for validity: a fetched suite inflates VS, and the canary scan only catches a fetch whose content reaches the transcript or the tree (census.py). S08 decision 4 names a per-task container as the strong form of isolation.

## Where

The places that start agent processes: `mini_loop.py` (direct arms), `run_cli.py` (CLI arms) and `run_roko.py` (Roko). The shared environment builder is `agent_env.py`.

## Current state

At 7fa54b873 agent processes have an allowlisted environment and no network restriction.

## Plan

1. **Deny network to agent processes, except the provider path.** Use a per-task container with network off, or `sandbox-exec` (macOS) or `unshare -n` / bwrap (Linux). The model calls go through the metering proxy on loopback, which the sandbox allows.
2. **Share the sandbox with the census.** gap-8c3752 needs the same one to run `hidden.py` without exposing the secret.
3. **Record the policy.** Store it in each run record's provenance.
4. **Test.** Add `test_an_agent_shell_cannot_reach_the_network`: a fake agent's `curl` to a local non-proxy port fails, and the proxy still works.

## Done when

- [ ] No agent process in any arm can open a connection other than to the metering proxy.
- [ ] The `[[verify]]` command passes.

## Notes

- Decide with gap-8c3752. One sandbox should serve both the agent run and the census.
- 2026-09-29 (wk-bench-fix2): the same isolation decision covers the environment. Any process of the same user exposes the environment it started with (`ps -E -ax` on macOS lists every such process's), so an agent can read the credentials of the driver, the proxy or any other process of that user, however well the driver scrubs its own. Only a separate uid or a container per task closes this. Decide it together with the network sandbox (and gap-8c3752's census sandbox).
- 2026-09-30: on Linux the benchmark has no sandbox at all yet (gap-29ac83): bubblewrap needs user namespaces, which Ubuntu runners restrict, and records there say `sandbox: none`. The network sandbox here needs the same Linux mechanism.
