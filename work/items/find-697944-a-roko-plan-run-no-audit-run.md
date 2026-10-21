+++
id = "find-697944"
kind = "finding"
title = "A roko plan run --no-audit run's S01 manifest lists only learning_frozen in ablation_flags, so Roko's own record does not say its audits were off"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-327242 worker (claude-agent), 2026-10-20: Pilot B s01 copies at c962955af"
discovered_from = "gap-327242"
anchors = ["crates/roko-cli/src/graph_execution/run_manifest.rs"]
links = { depends_on = [], blocks = [], related = ["gap-327242", "gap-d76b1b", "gap-c7c462"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'audit_off' crates/roko-cli/src/graph_execution/run_manifest.rs && cargo test -p roko-cli --lib -- run_manifest"
+++

## Problem

A `roko plan run --no-audit` run's S01 manifest (`.roko/runs/<run>/manifest.json`) records
`experiment.ablation_flags = ["learning_frozen"]` and nothing that says its audits were off. `--no-audit` sets
`[audit] enabled = false` for the run (`apply_run_switches`, gap-d76b1b), but the manifest's flags come only from
`config.learning.frozen`. `invocations[].args_sha256` hashes the argv, so the switch can be recovered only by
re-hashing a guessed command line.

Seen in Pilot B (gap-327242) on 2026-10-20, at roko c962955af: `PILOT-B.abandoned/roko-fixed-s1-1/s01/F4-l5-0002.s1/
runs/graph-plan-*/manifest.json`.

## Why it matters

Audits are on by default since 22ac6f06b. Whether a run audited, and could therefore have dispatched other models, is
now a property of the run that S01 readers need: G2's frozen-loop census reads `ablation_flags`, and so would any
check that a pinned arm ran only its pin. The bench records the switch in its own config (the arm file's
`[roko] audit = false`) and in the transcript's argv, but Roko's own record does not.

## Where

- `crates/roko-cli/src/graph_execution/run_manifest.rs`: where `ablation_flags` is filled.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::apply_run_switches`.

## Current state

Only `learning_frozen` reaches `ablation_flags`; `--no-audit` and `--no-holdout` do not.

## Plan

1. Add `audit_off` when `config.audit.enabled` is false for the run, and `holdout_off` for `--no-holdout`
   (`experiments.maximize`), with a test on the written manifest.

## Done when

- [ ] A `--no-audit` run's manifest lists `audit_off`: the `[[verify]]` test.

## Notes

- Found by gap-327242's worker on 2026-10-20.
