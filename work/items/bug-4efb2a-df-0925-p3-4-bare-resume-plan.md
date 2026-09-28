+++
id = "bug-4efb2a"
kind = "bug"
title = "DF-0925 P3-4: Bare `--resume-plan` fails for multi-plan runs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs"
anchors = ["main.rs:2028", "graph_checkpoint.rs:33 DEFAULT_RUNNER_RESUME_PATH", "graph_checkpoint.rs:508"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The clap default .roko/state/state-snapshot.json is not mapped to the graph checkpoint root (only executor.json is special-cased), so it is treated as a single-plan manifest and bails; the module doc claims otherwise.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P3-4. Bare `--resume-plan` is broken for multi-plan runs`

How to verify: Run plan run <dir> --resume-plan with no path on a multi-plan set.
