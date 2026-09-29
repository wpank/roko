+++
id = "gap-308373"
kind = "gap"
title = "A same-uid agent can read the ViabilityBench secret file silently, and nothing detects it"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-secret's report on gap-a8a160)"
anchors = ["benchmarks/viabilitybench/driver/secret.py", "benchmarks/viabilitybench/driver/census.py", "benchmarks/viabilitybench/driver/agent_env.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-a8a160", "gap-8c3752", "bug-979a06"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_silent_read_of_the_secret_file_is_detected' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_a_silent_read_of_the_secret_file_is_detected -q"
+++

## Problem

gap-a8a160 moved `VB_SECRET` into a driver-only file: mode 0600, in a 0700 directory, by default `~/.config/viabilitybench/secret`. The census looks for the secret and its canary wherever an agent could have put them.

Agents still run as the driver's user, though, so an agent can open the file by its absolute path. A read that leaves no trace goes unseen: one that never prints the secret or the canary, and never names the path in the transcript. With the secret, an agent can derive every hidden case, since they are HMAC(secret, instance_id).

`secret.py`'s docstring (:22-26) and `agent_env.py` (:19-22) state this limit. gap-8c3752 covers a different window, the census, when `hidden.py` runs agent code while the secret file is readable. It doesn't cover the agent's own run.

## Why it matters

Pilot benchmark (epic spec-567e52): SC4 ("hidden tests stay hidden") holds only against careless or honest agents. A published result needs either prevention or detection of a deliberate read.

## Where

- `driver/secret.py`: the file's rules, `preflight`, `check`.
- `driver/census.py`: the leak checks that mark a run `leak_suspected`.
- The `vb run` loop that runs agents and then the census.

## Current state

At BASE (4315add32), a silent read can't be detected. The file stays at 0600 for the whole run.

## Plan

There are two options, and they can be combined:

- **(a) Prevent.** Run each task in a container, or under a sandbox-exec (macOS) or bwrap (Linux) profile, that can't see the secret's directory (S08 decision 4). This is the strong form, and gap-8c3752 needs it too.
- **(b) Detect with a tripwire.** Keep the file at mode 000 while agents run, and make it readable (0400) only while `hidden.py` runs.
  - A same-uid agent must `chmod` the file to read it, and that changes the file's ctime, which a normal user can't reset.
  - The driver records the file's mode and ctime before each task and after it, and marks the run `leak_suspected` when either changed.
  - It is cheap and detects rather than prevents. The census window stays with gap-8c3752.

Recommended: implement (b) now, and leave (a) with S08 decision 4. Then:

1. Add the mode and ctime checks to `vb run` and the census, and record the tripwire's state in the run's provenance.
2. Add `test_a_silent_read_of_the_secret_file_is_detected`: a fake agent that chmods the file and reads it without printing anything makes the run `leak_suspected`.

## Done when

- [ ] A silent read of the secret file during an agent run marks the run `leak_suspected` (or is impossible, under option a).
- [ ] The `[[verify]]` command passes.

## Notes

- After any `leak_suspected` run, rotate the secret (`secret.py init` on a moved-aside file), as secret.py already says.
