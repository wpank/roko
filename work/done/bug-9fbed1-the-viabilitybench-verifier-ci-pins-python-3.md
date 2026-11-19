+++
id = "bug-9fbed1"
kind = "bug"
title = "The ViabilityBench verifier CI pins Python 3.12.8, which Ubuntu 26.04 runners lack, so all three of its jobs fail at setup-python on release PR #89"
status = "done"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["ci"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "4d724ae85"
source = "round v8: the lead's CI drift job for release PR #89 (run 44516751223)"
discovered_from = "agent:team-lead (round v8)"
anchors = [".github/workflows/viabilitybench-ci.yml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'python-version: \"3.12\"' .github/workflows/viabilitybench-ci.yml && ! grep -qE 'python-version: \"?3\\.[0-9]+\\.[0-9]+' .github/workflows/*.yml"

[closed]
at = 2026-11-19
at_ts = "2026-11-19T11:46:59Z"
commit = "4d724ae85"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "S"
forced = false
evidence = "viabilitybench-ci.yml pins python-version \"3.12\" in its three jobs instead of 3.12.8, which Ubuntu 26.04 runners lack (run 44516751223); requirements.lock is --universal for Python 3.11+, so no patch release was needed. Red first: the verify grep failed before the change; it passes now, and no workflow pins an exact patch of a runner-provided toolchain (Rust comes from rustup, Node by major)."
+++

## Problem

The ViabilityBench verifier CI (`.github/workflows/viabilitybench-ci.yml`) fails in all three of its jobs on release PR
#89, run 44516751223. `actions/setup-python@v5` says "The version '3.12.8' with architecture 'x64' was not found for
Ubuntu 26.04": `ubuntu-latest` is now 26.04, whose Python manifest has no 3.12.8.

The pin, from 8640a17f9 ("pins Python 3.12.8"), was for reproducibility, not a need:
- the venv installs `requirements.lock`, compiled `--universal --python-version 3.11` with hashes, so any Python
  3.11 or later installs it;
- the benchmark's docs name no patch release.

No other workflow pins an exact patch of a toolchain the runner image must hold. The Rust toolchains
(`dtolnay/rust-toolchain@1.96.1`) come from rustup, which serves every release, and Node is pinned by major (22).

## Why it matters

- Will wants GitHub up to date, and #89 can't go green while three of its jobs fail at setup.
- Goal: release.

## Where

- `.github/workflows/viabilitybench-ci.yml`: the three `python-version` pins.

## Plan

1. Pin `python-version: "3.12"`, the latest 3.12 patch the runner image provides, in all three jobs.

## Done when

- The `[[verify]]` command passes, and the workflow's jobs pass setup on the PR.

## Notes

- The lead asked for this (round v8), for release PR #89.
