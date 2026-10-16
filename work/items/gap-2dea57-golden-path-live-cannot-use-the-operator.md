+++
id = "gap-2dea57"
kind = "gap"
title = "golden_path_live cannot use the operator's ~/.roko/.env: it hands roko an empty temp HOME and expects provider keys exported into the test's environment"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-c013c5 golden-path live runs (2026-10-16)"
discovered_from = "gap-c013c5"
anchors = ["crates/roko-cli/tests/golden_path_acceptance.rs::golden_path_live", "crates/roko-cli/tests/golden_path_acceptance.rs::plan_command"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "gap-c013c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'ROKO_GOLDEN_PATH_ENV_FILE' crates/roko-cli/tests/golden_path_acceptance.rs && cargo test -p roko-cli --test golden_path_acceptance --no-run"
+++

## Problem

`golden_path_live` (backlog 3117) gives roko an empty temp `HOME`, so that `[task.accept]` never writes the real
`~/.roko/accept/`. As a side effect, roko loads no `~/.roko/.env`, and the test expects the provider keys to be
exported into its own environment. But `~/.roko/.env` is where roko keeps keys: `load_startup_env_files` loads it at
startup and records the names as dotenv-supplied, so gate and agent children strip them. An operator whose keys live
only there has to copy them into the shell to run the test.

The 2026-10-16 live runs (gap-c013c5) ran under a rule never to read or copy a key. So they ran the test's steps from a
script that made the temp HOME's `.roko/.env` a symlink to the operator's file
(`tmp/status-quo/2026-10-16-live/evidence/tools/harness.py` in the main checkout).

A second, smaller mismatch: the report's `run_id` is the per-plan run record (`graph-golden-path-<uuid>`). That id
does not match the `graph-(run-)?[0-9a-f]+` pattern in 3118's verify. Only the Graph run id in `.roko/events.jsonl`
(`graph-<uuid>`) matches it.

## Why it matters

The live acceptance run should be repeatable by any operator without handling keys by hand (gap-f30b8e, spec-f09094).
It also avoids keys ending up in a shell environment that every child process inherits.

## Where

`crates/roko-cli/tests/golden_path_acceptance.rs`: `golden_path_live`, `plan_command` (sets `HOME`, removes `ROKO_*`),
and `live_report` (the report's `run_id`).

## Plan

1. Add `ROKO_GOLDEN_PATH_ENV_FILE`. When it is set, symlink that file as `<temp home>/.roko/.env` before the run,
   without reading it, so roko loads the keys itself.
2. Put the Graph run id in the report next to the run record id. It is in `.roko/events.jsonl`, and in the JSON
   summary if the summary carries it.
3. Say both in the test's doc comment.

## Done when

- The `[[verify]]` command passes.

## Notes

- Keep the temp HOME: it is what keeps `[task.accept]` out of the real `~/.roko/accept/`.
