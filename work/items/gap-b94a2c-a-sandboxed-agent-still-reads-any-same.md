+++
id = "gap-b94a2c"
kind = "gap"
title = "A sandboxed agent still reads any same-user process's start-up environment with sysctl(KERN_PROCARGS2), and no sandbox-exec rule stops it"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "L"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-0bd49a worker (claude-agent), 2026-10-16"
discovered_from = "gap-0bd49a's notes of 2026-09-29 and 2026-10-02 (the environment half of its isolation question)"
anchors = ["benchmarks/viabilitybench/families/common/sandbox.py::profile", "benchmarks/viabilitybench/driver/agent_env.py", "benchmarks/viabilitybench/driver/test_secret.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-0bd49a", "gap-29ac83", "gap-308373"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_agent_cannot_read_another_process_environment' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_an_agent_cannot_read_another_process_environment -q"
+++

## Problem

On macOS, sandboxed agent code can still read the start-up environment, and the argv, of any other process of the same
user that isn't an Apple system binary. It does this with `sysctl({CTL_KERN, KERN_PROCARGS2, pid})`. The setuid `ps`
cannot start inside the sandbox, but this call needs no setuid. `driver/test_secret.py`'s `PROCARGS` probe already
uses it.

Checked on macOS 26.4 (Darwin 25.4.0) on 2026-10-16. The target was a non-Apple interpreter started with a marker
in its environment, and the probe printed only whether the marker was visible. These profiles all leave the
environment readable:

- `(allow default)`
- `+ (deny network-outbound)`
- `+ (deny process-info*)`
- `+ (deny process-info* (target others))`
- `+ (deny sysctl-read (sysctl-name "kern.procargs2"))`
- `+ (deny sysctl-read (sysctl-name-prefix "kern.procargs"))`

A blanket `(deny sysctl-read)` blocks the read but also stops Python from starting: `import ctypes` fails.

So any credential in the environment of a shell, an editor or another session of the operator reaches every agent
(through the agent's shell, or code a truth suite runs). This holds even though the driver re-execs itself scrubbed
(`agent_env.exec_scrubbed`) and keeps keys only in the proxy's memory.

## Why it matters

This is the pilot benchmark's isolation (epic spec-567e52, S08 SC4). The README tells operators to run from a
session that exports no credential, so this is a documented limit rather than a silent one. But nothing enforces or
detects it. gap-0bd49a's 2026-09-29 and 2026-10-02 notes named it, and it is the second half of that item's isolation
question.

## Where

- `benchmarks/viabilitybench/families/common/sandbox.py::profile`: the only confinement agent code gets on macOS.
- `benchmarks/viabilitybench/driver/agent_env.py`: its "Limit." paragraph documents the gap.
- `benchmarks/viabilitybench/driver/test_secret.py`: `PROCARGS` and `PS_PROBE`, the probes that read a parent's
  environment.

## Current state

No host-only rule found closes it. Only a different uid, or a container per task (S08 decision 4), hides another
process's environment from the agent. Both are design decisions with setup cost on Will's Mac: a dedicated user with
sudo rules, or a container runtime with an image per family toolchain and the CLI arms' credentials inside it.

## Plan

1. Decide the mechanism (Will): a separate uid for agent processes and the census's runs of agent code, a container
   per task, or accept and document it (today's state) with a preflight that refuses a run while a same-user process
   exports a known credential name.
2. Implement it behind `common.sandbox`'s backend seam (`BACKENDS`, gap-0bd49a), so the census shares it.
3. Add `test_an_agent_cannot_read_another_process_environment` in `driver/test_secret.py`. A same-user process
   started with a marker in its environment, and the `PROCARGS` probe run as an agent command must not see the
   marker, with an unconfined control that does.

## Done when

- [ ] Sandboxed agent code cannot read another same-user process's environment, or the run refuses to start while
  one exports a credential.
- [ ] The `[[verify]]` command passes.

## Notes

- Never print a real process's environment while testing this: start your own target with a harmless marker, and
  print only whether the marker was seen.
- Related: gap-29ac83 (Linux sandbox), which needs the same backend seam.
