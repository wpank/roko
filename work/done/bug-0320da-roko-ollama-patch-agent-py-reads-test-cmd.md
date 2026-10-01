+++
id = "bug-0320da"
kind = "bug"
title = "roko-ollama-patch-agent.py reads test_cmd from the payload, which is now hidden, so its self-check falls back to true"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["demo/demo-resources"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a53a53c70"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-960ab1 at b482d7830)"
anchors = ["demo/demo-resources/coding-agent-benchmarks/roko-ollama-patch-agent.py"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-28becc", "bug-960ab1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'or \"true\"' demo/demo-resources/coding-agent-benchmarks/roko-ollama-patch-agent.py"

[closed]
at = 2026-09-30
commit = "a53a53c70"
by = "commit trailer"
evidence = "a53a53c70: the agent validates with --validate-cmd / ROKO_BENCH_VALIDATE_CMD, else the repo's visible test*.py via python -m unittest discover; with neither it exits 2 before running roko. Verify passes; the discovered check exits 1 on the smoke fixture's bug and 0 after the fix."
+++

## Problem

`demo/demo-resources/coding-agent-benchmarks/roko-ollama-patch-agent.py` takes its validation command from the task payload: `test_cmd = instance.get("test_cmd") or instance.get("test_command") or "true"` (:94). It writes it into the agent's `roko.toml` (:69-70) and prompt (:99). bug-28becc stopped exposing the test command in the payload, so the command is always `true`, and the agent's self-check passes whatever it does.

## Why it matters

Release: a demo agent whose own check can't fail teaches the wrong lesson, and its results look self-verified. p3.

## Where

The script's command lookup.

## Plan

1. Use the visible checks the benchmark still gives agents (if any), or fail loudly when there's no validation command, instead of substituting `true`.

## Done when

- [x] The script never runs with a vacuous self-check.
- [x] The `[[verify]]` command passes.
