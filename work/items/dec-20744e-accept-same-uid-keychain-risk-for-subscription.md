+++
id = "dec-20744e"
kind = "decision"
title = "Accept same-uid keychain risk for subscription arms, or containerize them (S08 decision 4)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "gap-3cfe4f (wave 4, detection only)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/census.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

In ViabilityBench's `fd_claude` arm, the agent's shell runs as the same macOS user as Claude Code, so it can read the
subscription login (the keychain entry) that Claude Code itself needs. gap-3cfe4f (closed in db49bfd1d) showed that
prevention without containers is infeasible on macOS:
- `security` isn't setuid, and the keychain is readable by the same user.
- The whole `claude` process tree shares one sandbox for the egress rule, so a keychain deny there would also stop
  Claude Code's own login.
- macOS refuses a nested sandbox, so the agent's shell commands can't be confined on their own.

What's in place now is detection. The census marks a run `leak_suspected` when an agent shell names a keychain
operation, the `security` wrapper or a keychain path, and `vb run --credential-fingerprint` can match the token's
fingerprint in transcripts and outputs without holding the token.

## Why it matters

Pilot B's and LOG1's `fd_claude` runs use the subscription. A leaked login could end up in transcripts or published
bundles.

## Where

`benchmarks/viabilitybench/driver/run_cli.py`, `driver/census.py`, `families/common/sandbox.py`; S08 decision 4.

## Current state

Detection only (gap-3cfe4f). Same-uid reads of other secrets are the same family of risk (gap-308373).

## Plan

Will chooses:
- **(a) Accept the same-uid risk on a personal machine (recommended for now).** Run the subscription arms from a login
  that holds nothing else an agent shouldn't read. Rotate the Claude Code credential after any `leak_suspected` run,
  as gap-308373 does for other secrets. Keep the detector as the floor and pass `--credential-fingerprint` on paid or
  subscription campaigns.
- **(b) Containers per task (S08 decision 4)** for real prevention: a container runtime on the Mac, an image per
  family toolchain, and Claude Code's credentials inside the container. About a week of work.

## Done when

Will's choice is recorded here. Under (a), Pilot B's runbook passes `--credential-fingerprint` and says how to
rotate. Under (b), an item is filed for the container sandbox.

## Notes

- Raised by the coordinator after gate 4b, 2026-10-02.
